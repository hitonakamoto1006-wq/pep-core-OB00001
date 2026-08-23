use crate::PEPDEX::aggregator::{
    AggregatorError,
    ExecutionAggregator,
    ParentExecution,
};
use crate::PEPDEX::balance::{
    BalanceError,
    BalanceLedger,
};
use crate::PEPDEX::execution::{
    Execution,
    ExecutionTracker,
};
use crate::PEPDEX::executor::{
    ExecutionResult,
    Executor,
    ExecutorError,
};
use crate::PEPDEX::market::Market;
use crate::PEPDEX::market_data::MarketDataEngine;
use crate::PEPDEX::order::{
    ExecutionPlan,
    ParentOrder,
};
use crate::PEPDEX::order_store::{
    OrderStore,
    OrderStoreError,
};
use crate::PEPDEX::reconciliation::{
    ReconciliationEngine,
};
use crate::PEPDEX::reservation::{
    ReservationError,
    ReservationId,
    ReservationManager,
};
use crate::PEPDEX::risk::{
    RiskEngine,
    RiskError,
};
use crate::PEPDEX::router::{
    Router,
    RouterError,
};
use crate::PEPDEX::settlement::{
    SettlementEngine,
    SettlementError,
    SettlementResult,
    SettlementStatus,
};
use crate::PEPDEX::splitter::{
    Splitter,
    SplitterError,
};
use crate::PEPDEX::venue::{
    VenueOrderSide,
};

/// ============================================================
/// ORDER LIFECYCLE
/// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderLifecycle {
    Received,

    RiskChecked,

    Reserved,

    MarketData,

    Routed,

    Split,

    Executing,

    Aggregated,

    Settled,

    PartiallySettled,

    Failed,
}

/// ============================================================
/// MANAGED ORDER
///
/// Runtime representation of a parent order.
///
/// This object exists while the order is being processed.
/// Persistent state is written through OrderStore.
/// ============================================================

#[derive(Debug)]
pub struct ManagedOrder {
    pub order: ParentOrder,

    pub lifecycle:
        OrderLifecycle,

    pub reservation_id:
        Option<ReservationId>,

    pub execution_plan:
        Option<ExecutionPlan>,

    pub execution_result:
        Option<ExecutionResult>,

    pub executions:
        Vec<Execution>,

    pub parent_execution:
        Option<ParentExecution>,

    pub settlement:
        Option<SettlementResult>,
}

impl ManagedOrder {
    pub fn new(
        order: ParentOrder,
    ) -> Self {
        Self {
            order,

            lifecycle:
                OrderLifecycle::Received,

            reservation_id:
                None,

            execution_plan:
                None,

            execution_result:
                None,

            executions:
                Vec::new(),

            parent_execution:
                None,

            settlement:
                None,
        }
    }

    pub fn is_finished(
        &self,
    ) -> bool {
        matches!(
            self.lifecycle,
            OrderLifecycle::Settled
                | OrderLifecycle::PartiallySettled
                | OrderLifecycle::Failed
        )
    }
}

/// ============================================================
/// ORDER MANAGER
///
/// Main PEPDEX execution kernel.
///
/// Parent order:
///
///     Risk
///       ↓
///     Reserve
///       ↓
///     Market Data
///       ↓
///     Router
///       ↓
///     Splitter
///       ↓
///     Executor
///       ↓
///     Aggregator
///       ↓
///     Settlement
///       ↓
///     Balance
///
/// OrderStore persists state at every important boundary.
/// ============================================================

pub struct OrderManager {
    pub market_data:
        MarketDataEngine,

    pub risk:
        RiskEngine,

    pub router:
        Router,

    pub splitter:
        Splitter,

    pub executor:
        Executor,

    pub aggregator:
        ExecutionAggregator,

    pub settlement:
        SettlementEngine,

    pub balances:
        BalanceLedger,

    pub reservations:
        ReservationManager,

    pub orders:
        OrderStore,

    pub reconciliation:
        ReconciliationEngine,
}

impl OrderManager {
    /// Create an OrderManager using a supplied persistent
    /// order store.
    pub fn new(
        market_data:
            MarketDataEngine,

        router:
            Router,

        splitter:
            Splitter,

        executor:
            Executor,

        balances:
            BalanceLedger,

        orders:
            OrderStore,
    ) -> Self {
        let reservations =
            ReservationManager::new(
                balances.clone()
            );

        Self {
            market_data,

            risk:
                RiskEngine::new(),

            router,

            splitter,

            executor,

            aggregator:
                ExecutionAggregator::new(),

            settlement:
                SettlementEngine::new(),

            balances,

            reservations,

            orders,

            reconciliation:
                ReconciliationEngine::new(),
        }
    }

    /// ========================================================
    /// PROCESS PARENT ORDER
    /// ========================================================

    pub fn process(
        &self,
        order: ParentOrder,
        market: &Market,
    ) -> Result<
        ManagedOrder,
        OrderManagerError,
    > {
        let mut managed =
            ManagedOrder::new(order);

        /*
         * ----------------------------------------------------
         * 1. RECEIVE
         * ----------------------------------------------------
         *
         * Persist immediately.
         *
         * If the process dies after this point,
         * PEPDEX still knows that the order existed.
         */

        self.persist(
            &managed
        )?;

        /*
         * ----------------------------------------------------
         * 2. RISK CHECK
         * ----------------------------------------------------
         */

        if let Err(error) =
            self.risk.validate(
                &managed.order,
                &managed
                    .order
                    .constraints,
            )
        {
            managed.lifecycle =
                OrderLifecycle::Failed;

            self.persist_error(
                &managed,
                format!("{:?}", error),
            )?;

            return Err(
                OrderManagerError::Risk(
                    error
                )
            );
        }

        managed.lifecycle =
            OrderLifecycle::RiskChecked;

        self.persist(
            &managed
        )?;

        /*
         * ----------------------------------------------------
         * 3. DETERMINE RESERVATION ASSET
         * ----------------------------------------------------
         */

        let reservation_asset =
            match managed.order.side {
                VenueOrderSide::Buy => {
                    managed
                        .order
                        .market
                        .quote
                        .clone()
                }

                VenueOrderSide::Sell => {
                    managed
                        .order
                        .market
                        .base
                        .clone()
                }
            };

        /*
         * ----------------------------------------------------
         * 4. DETERMINE RESERVATION AMOUNT
         * ----------------------------------------------------
         */

        let reservation_amount =
            match self
                .calculate_reservation_amount(
                    &managed.order
                )
            {
                Ok(amount) => amount,

                Err(error) => {
                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(error);
                }
            };

        /*
         * ----------------------------------------------------
         * 5. RESERVE USER BALANCE
         * ----------------------------------------------------
         */

        let reservation_id =
            match self.reservations.reserve(
                &managed.order,
                reservation_asset,
                reservation_amount,
            ) {
                Ok(id) => id,

                Err(error) => {
                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(
                        OrderManagerError
                            ::Reservation(error)
                    );
                }
            };

        managed.reservation_id =
            Some(
                reservation_id
            );

        managed.lifecycle =
            OrderLifecycle::Reserved;

        self.persist(
            &managed
        )?;

        /*
         * ----------------------------------------------------
         * 6. MARKET DATA
         * ----------------------------------------------------
         */

        managed.lifecycle =
            OrderLifecycle::MarketData;

        self.persist(
            &managed
        )?;

        let book =
            match self.market_data.snapshot(
                &managed.order.market
            ) {
                Ok(book) => book,

                Err(error) => {
                    self.release_reservation(
                        &managed
                    );

                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(
                        OrderManagerError
                            ::MarketData(error)
                    );
                }
            };

        /*
         * ----------------------------------------------------
         * 7. ROUTER
         * ----------------------------------------------------
         *
         * Parent order becomes an execution plan.
         *
         * Example:
         *
         * 100 PEP
         *
         * → Aster    40
         * → Binance  35
         * → OKX      25
         */

        let plan =
            match self.router.route(
                &managed.order,
                &book,
            ) {
                Ok(plan) => plan,

                Err(error) => {
                    self.release_reservation(
                        &managed
                    );

                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(
                        OrderManagerError
                            ::Routing(error)
                    );
                }
            };

        managed.lifecycle =
            OrderLifecycle::Routed;

        self.persist(
            &managed
        )?;

        /*
         * ----------------------------------------------------
         * 8. SPLIT
         * ----------------------------------------------------
         */

        let plan =
            match self.splitter.split(
                &plan,
                market,
            ) {
                Ok(plan) => plan,

                Err(error) => {
                    self.release_reservation(
                        &managed
                    );

                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(
                        OrderManagerError
                            ::Splitting(error)
                    );
                }
            };

        managed.execution_plan =
            Some(
                plan.clone()
            );

        managed.lifecycle =
            OrderLifecycle::Split;

        /*
         * IMPORTANT:
         *
         * At this point child orders are persisted
         * BEFORE external execution begins.
         *
         * Therefore a crash immediately after this
         * point still leaves the execution plan.
         */

        self.persist(
            &managed
        )?;

        /*
         * ----------------------------------------------------
         * 9. EXECUTE EXTERNAL ORDERS
         * ----------------------------------------------------
         */

        managed.lifecycle =
            OrderLifecycle::Executing;

        self.persist(
            &managed
        )?;

        let execution_result =
            match self.executor.execute(
                &plan
            ) {
                Ok(result) => result,

                Err(error) => {
                    /*
                     * IMPORTANT:
                     *
                     * Executor failure does NOT necessarily
                     * mean no external order was created.
                     *
                     * Therefore we persist FAILED and leave
                     * reconciliation capable of recovering
                     * external executions.
                     */

                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(
                        OrderManagerError
                            ::Execution(error)
                    );
                }
            };

        managed.execution_result =
            Some(
                execution_result
            );

        /*
         * ----------------------------------------------------
         * 10. NORMALIZE VENUE EXECUTIONS
         * ----------------------------------------------------
         */

        let execution_result =
            managed
                .execution_result
                .as_ref()
                .expect(
                    "execution result missing"
                );

        let mut tracker =
            ExecutionTracker::new();

        for child_execution
            in &execution_result.executions
        {
            let execution =
                Execution::from_venue_order(
                    &child_execution
                        .child_order,

                    &child_execution
                        .venue_order,
                );

            tracker.add(
                execution
            );
        }

        managed.executions =
            tracker
                .all()
                .to_vec();

        /*
         * Persist external order IDs immediately.
         *
         * This is critical.
         *
         * If process dies after this point,
         * reconciliation knows which external orders
         * to query.
         */

        self.persist(
            &managed
        )?;

        /*
         * No successful execution.
         */

        if managed.executions.is_empty() {
            self.release_reservation(
                &managed
            );

            managed.lifecycle =
                OrderLifecycle::Failed;

            self.persist_error(
                &managed,
                "No successful execution",
            )?;

            return Err(
                OrderManagerError
                    ::NoSuccessfulExecution
            );
        }

        /*
         * ----------------------------------------------------
         * 11. AGGREGATE
         * ----------------------------------------------------
         */

        let parent_execution =
            match self.aggregator.aggregate(
                &managed.order,
                &managed.executions,
            ) {
                Ok(execution) => execution,

                Err(error) => {
                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(
                        OrderManagerError
                            ::Aggregation(error)
                    );
                }
            };

        managed.parent_execution =
            Some(
                parent_execution
            );

        managed.lifecycle =
            OrderLifecycle::Aggregated;

        self.persist(
            &managed
        )?;

        /*
         * ----------------------------------------------------
         * 12. FINAL RISK CHECK
         * ----------------------------------------------------
         *
         * Initial risk check:
         *
         *     "May this order execute?"
         *
         * Final risk check:
         *
         *     "Did execution actually respect
         *      the user's constraints?"
         */

        let parent_execution =
            managed
                .parent_execution
                .as_ref()
                .expect(
                    "parent execution missing"
                );

        let reference_price =
            if managed
                .order
                .constraints
                .max_slippage_bps
                .is_some()
            {
                self.risk
                    .reference_price(
                        &managed.order,
                        &self.market_data,
                    )
                    .ok()
            } else {
                None
            };

        if let Err(error) =
            self.risk
                .validate_execution_cost(
                    &managed.order,

                    &managed
                        .order
                        .constraints,

                    parent_execution
                        .total_cost
                        .saturating_add(
                            parent_execution
                                .total_venue_fee
                        ),

                    reference_price,
                )
        {
            managed.lifecycle =
                OrderLifecycle::Failed;

            self.persist_error(
                &managed,
                format!("{:?}", error),
            )?;

            return Err(
                OrderManagerError::Risk(
                    error
                )
            );
        }

        /*
         * ----------------------------------------------------
         * 13. CONSUME RESERVATION
         * ----------------------------------------------------
         */

        let consumed =
            self.calculate_consumed_amount(
                &managed.order,
                parent_execution,
            );

        if let Some(
            reservation_id
        ) =
            &managed.reservation_id
        {
            if let Err(error) =
                self.reservations.consume(
                    reservation_id,
                    consumed,
                )
            {
                managed.lifecycle =
                    OrderLifecycle::Failed;

                self.persist_error(
                    &managed,
                    format!("{:?}", error),
                )?;

                return Err(
                    OrderManagerError
                        ::Reservation(error)
                );
            }
        }

        /*
         * ----------------------------------------------------
         * 14. SETTLEMENT
         * ----------------------------------------------------
         */

        let settlement =
            match self.settlement.settle(
                &managed.order,
                parent_execution,
            ) {
                Ok(result) => result,

                Err(error) => {
                    /*
                     * External execution already happened.
                     *
                     * DO NOT unlock the consumed amount.
                     *
                     * Reconciliation must resolve this.
                     */

                    managed.lifecycle =
                        OrderLifecycle::Failed;

                    self.persist_error(
                        &managed,
                        format!("{:?}", error),
                    )?;

                    return Err(
                        OrderManagerError
                            ::Settlement(error)
                    );
                }
            };

        /*
         * ----------------------------------------------------
         * 15. APPLY BALANCE DELTA
         * ----------------------------------------------------
         */

        if let Err(error) =
            self.balances
                .apply_settlement(
                    &settlement
                )
        {
            /*
             * External execution is real.
             *
             * Balance application failed.
             *
             * Do NOT release reservation here.
             *
             * This is a reconciliation case.
             */

            managed.lifecycle =
                OrderLifecycle::Failed;

            self.persist_error(
                &managed,
                format!("{:?}", error),
            )?;

            return Err(
                OrderManagerError
                    ::Balance(error)
            );
        }

        /*
         * ----------------------------------------------------
         * 16. RELEASE UNUSED RESERVATION
         * ----------------------------------------------------
         */

        if let Some(
            reservation_id
        ) =
            &managed.reservation_id
        {
            if let Err(error) =
                self.reservations
                    .release(
                        reservation_id
                    )
            {
                managed.lifecycle =
                    OrderLifecycle::Failed;

                self.persist_error(
                    &managed,
                    format!("{:?}", error),
                )?;

                return Err(
                    OrderManagerError
                        ::Reservation(error)
                );
            }
        }

        /*
         * ----------------------------------------------------
         * 17. FINAL STATE
         * ----------------------------------------------------
         */

        managed.lifecycle =
            match settlement.status {
                SettlementStatus::Settled => {
                    OrderLifecycle::Settled
                }

                SettlementStatus
                    ::PartiallySettled => {
                    OrderLifecycle
                        ::PartiallySettled
                }

                SettlementStatus
                    ::Cancelled
                | SettlementStatus::Failed => {
                    OrderLifecycle::Failed
                }
            };

        managed.settlement =
            Some(
                settlement
            );

        /*
         * Final persistent checkpoint.
         */

        self.persist(
            &managed
        )?;

        Ok(managed)
    }

    /// ========================================================
    /// RESERVATION CALCULATION
    /// ========================================================

    fn calculate_reservation_amount(
        &self,
        order: &ParentOrder,
    ) -> Result<
        i128,
        OrderManagerError,
    > {
        match order.side {
            VenueOrderSide::Sell => {
                /*
                 * SELL reserves base asset.
                 */
                Ok(
                    order.quantity
                        as i128
                )
            }

            VenueOrderSide::Buy => {
                /*
                 * LIMIT BUY:
                 *
                 * quantity × limit price
                 */
                if let Some(price) =
                    order.limit_price
                {
                    let amount =
                        order
                            .quantity
                            .checked_mul(
                                price
                            )
                            .ok_or(
                                OrderManagerError
                                    ::ReservationOverflow
                            )?;

                    return Ok(
                        amount as i128
                    );
                }

                /*
                 * MARKET BUY:
                 *
                 * max_spend is mandatory.
                 */
                order
                    .constraints
                    .max_spend
                    .map(|amount| {
                        amount as i128
                    })
                    .ok_or(
                        OrderManagerError
                            ::MarketBuyRequiresMaxSpend
                    )
            }
        }
    }

    /// ========================================================
    /// ACTUAL CONSUMPTION
    /// ========================================================

    fn calculate_consumed_amount(
        &self,
        order: &ParentOrder,
        execution: &ParentExecution,
    ) -> i128 {
        match order.side {
            VenueOrderSide::Buy => {
                /*
                 * BUY consumes quote.
                 */
                execution
                    .total_cost
                    .saturating_add(
                        execution
                            .total_venue_fee
                    ) as i128
            }

            VenueOrderSide::Sell => {
                /*
                 * SELL consumes base.
                 */
                execution
                    .filled_quantity
                    as i128
            }
        }
    }

    /// ========================================================
    /// RELEASE RESERVATION
    /// ========================================================

    fn release_reservation(
        &self,
        managed: &ManagedOrder,
    ) {
        if let Some(
            reservation_id
        ) =
            &managed.reservation_id
        {
            let _ =
                self.reservations
                    .release(
                        reservation_id
                    );
        }
    }

    /// ========================================================
    /// PERSIST
    /// ========================================================

    fn persist(
        &self,
        managed: &ManagedOrder,
    ) -> Result<
        (),
        OrderManagerError,
    > {
        self.orders
            .save_managed(
                managed
            )
            .map_err(
                OrderManagerError
                    ::OrderStore
            )
    }

    /// ========================================================
    /// PERSIST ERROR
    /// ========================================================

    fn persist_error(
        &self,
        managed: &ManagedOrder,
        error: impl Into<String>,
    ) -> Result<
        (),
        OrderManagerError,
    > {
        /*
         * Try to save the current state first.
         */
        let _ =
            self.orders
                .save_managed(
                    managed
                );

        self.orders
            .set_error(
                &managed.order.id,
                error,
            )
            .map_err(
                OrderManagerError
                    ::OrderStore
            )
    }
}

/// ============================================================
/// ERRORS
/// ============================================================

#[derive(Debug)]
pub enum OrderManagerError {
    Risk(
        RiskError
    ),

    MarketData(
        crate::PEPDEX::venue::VenueError
    ),

    Routing(
        RouterError
    ),

    Splitting(
        SplitterError
    ),

    Execution(
        ExecutorError
    ),

    Aggregation(
        AggregatorError
    ),

    Settlement(
        SettlementError
    ),

    Balance(
        BalanceError
    ),

    Reservation(
        ReservationError
    ),

    OrderStore(
        OrderStoreError
    ),

    NoSuccessfulExecution,

    ReservationOverflow,

    MarketBuyRequiresMaxSpend,
}