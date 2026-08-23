use crate::PEPDEX::aggregator::{
    ParentExecution,
    ParentExecutionStatus,
};
use crate::PEPDEX::order::{
    OrderId,
    ParentOrder,
};
use crate::PEPDEX::venue::VenueOrderSide;

#[derive(Debug, Clone)]
pub struct SettlementResult {
    pub parent_order_id: OrderId,
    pub user: String,

    pub asset: String,
    pub quote_asset: String,

    pub asset_delta: i128,
    pub quote_delta: i128,

    pub execution_quantity: u64,
    pub execution_cost: u64,
    pub venue_fee: u64,

    pub status: SettlementStatus,
}

impl SettlementResult {
    pub fn is_successful(&self) -> bool {
        matches!(
            self.status,
            SettlementStatus::Settled
                | SettlementStatus::PartiallySettled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementStatus {
    Settled,
    PartiallySettled,
    Cancelled,
    Failed,
}

#[derive(Debug)]
pub enum SettlementError {
    ParentOrderMismatch,

    InvalidExecutionState,

    ZeroFilledQuantity,

    InsufficientSettlementData,
}

pub struct SettlementEngine;

impl SettlementEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn settle(
        &self,
        parent_order: &ParentOrder,
        execution: &ParentExecution,
    ) -> Result<SettlementResult, SettlementError> {
        if parent_order.id
            != execution.parent_order_id
        {
            return Err(
                SettlementError::ParentOrderMismatch
            );
        }

        if execution.filled_quantity == 0 {
            return Err(
                SettlementError::ZeroFilledQuantity
            );
        }

        let status =
            match execution.status {
                ParentExecutionStatus::Filled => {
                    SettlementStatus::Settled
                }

                ParentExecutionStatus::PartiallyFilled => {
                    SettlementStatus::PartiallySettled
                }

                ParentExecutionStatus::Cancelled => {
                    SettlementStatus::Cancelled
                }

                ParentExecutionStatus::Failed => {
                    SettlementStatus::Failed
                }

                ParentExecutionStatus::Pending
                | ParentExecutionStatus::Executing => {
                    return Err(
                        SettlementError::InvalidExecutionState
                    );
                }
            };

        let (
            asset_delta,
            quote_delta,
        ) = match parent_order.side {
            /*
             * BUY:
             *
             * User receives base asset.
             * User spends quote asset.
             */
            VenueOrderSide::Buy => {
                (
                    execution
                        .filled_quantity as i128,

                    -(
                        execution.total_cost
                            as i128
                    )
                    -
                    (
                        execution
                            .total_venue_fee
                            as i128
                    ),
                )
            }

            /*
             * SELL:
             *
             * User gives base asset.
             * User receives quote asset.
             */
            VenueOrderSide::Sell => {
                (
                    -(
                        execution
                            .filled_quantity
                            as i128
                    ),

                    (
                        execution.total_cost
                            as i128
                    )
                    -
                    (
                        execution
                            .total_venue_fee
                            as i128
                    ),
                )
            }
        };

        Ok(SettlementResult {
            parent_order_id:
                parent_order.id.clone(),

            user:
                parent_order.user.clone(),

            asset:
                parent_order
                    .market
                    .base
                    .clone(),

            quote_asset:
                parent_order
                    .market
                    .quote
                    .clone(),

            asset_delta,

            quote_delta,

            execution_quantity:
                execution.filled_quantity,

            execution_cost:
                execution.total_cost,

            venue_fee:
                execution.total_venue_fee,

            status,
        })
    }
}

impl Default for SettlementEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::aggregator::{
        ParentExecution,
        ParentExecutionStatus,
    };

    use crate::PEPDEX::market::MarketId;

    fn buy_order() -> ParentOrder {
        ParentOrder::new(
            "PARENT-BUY",
            "alice",
            MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            crate::PEPDEX::venue::VenueOrderType::Market,
            100,
            None,
        )
    }

    fn sell_order() -> ParentOrder {
        ParentOrder::new(
            "PARENT-SELL",
            "alice",
            MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Sell,
            crate::PEPDEX::venue::VenueOrderType::Market,
            100,
            None,
        )
    }

    fn filled_execution(
        order: &ParentOrder,
    ) -> ParentExecution {
        ParentExecution {
            parent_order_id:
                order.id.clone(),

            requested_quantity: 100,

            filled_quantity: 100,

            remaining_quantity: 0,

            total_cost: 60_200,

            total_venue_fee: 20,

            average_price:
                Some(602),

            status:
                ParentExecutionStatus::Filled,

            venue_executions:
                Vec::new(),
        }
    }

    #[test]
    fn buy_settlement_credits_base_asset() {
        let order =
            buy_order();

        let execution =
            filled_execution(&order);

        let engine =
            SettlementEngine::new();

        let result = engine
            .settle(
                &order,
                &execution,
            )
            .expect(
                "settlement failed"
            );

        /*
         * BUY:
         *
         * +100 PEP
         * -60,220 USDT
         */
        assert_eq!(
            result.asset,
            "PEP"
        );

        assert_eq!(
            result.quote_asset,
            "USDT"
        );

        assert_eq!(
            result.asset_delta,
            100
        );

        assert_eq!(
            result.quote_delta,
            -60_220
        );

        assert_eq!(
            result.status,
            SettlementStatus::Settled
        );
    }

    #[test]
    fn sell_settlement_credits_quote_asset() {
        let order =
            sell_order();

        let execution =
            filled_execution(&order);

        let engine =
            SettlementEngine::new();

        let result = engine
            .settle(
                &order,
                &execution,
            )
            .expect(
                "settlement failed"
            );

        /*
         * SELL:
         *
         * -100 PEP
         * +60,180 USDT
         */
        assert_eq!(
            result.asset_delta,
            -100
        );

        assert_eq!(
            result.quote_delta,
            60_180
        );

        assert_eq!(
            result.status,
            SettlementStatus::Settled
        );
    }

    #[test]
    fn partial_execution_is_partially_settled() {
        let order =
            buy_order();

        let execution =
            ParentExecution {
                parent_order_id:
                    order.id.clone(),

                requested_quantity: 100,

                filled_quantity: 60,

                remaining_quantity: 40,

                total_cost: 36_120,

                total_venue_fee: 12,

                average_price:
                    Some(602),

                status:
                    ParentExecutionStatus
                        ::PartiallyFilled,

                venue_executions:
                    Vec::new(),
            };

        let engine =
            SettlementEngine::new();

        let result = engine
            .settle(
                &order,
                &execution,
            )
            .expect(
                "settlement failed"
            );

        assert_eq!(
            result.asset_delta,
            60
        );

        assert_eq!(
            result.quote_delta,
            -36_132
        );

        assert_eq!(
            result.status,
            SettlementStatus
                ::PartiallySettled
        );
    }

    #[test]
    fn pending_execution_cannot_settle() {
        let order =
            buy_order();

        let execution =
            ParentExecution {
                parent_order_id:
                    order.id.clone(),

                requested_quantity: 100,

                filled_quantity: 50,

                remaining_quantity: 50,

                total_cost: 30_100,

                total_venue_fee: 10,

                average_price:
                    Some(602),

                status:
                    ParentExecutionStatus
                        ::Executing,

                venue_executions:
                    Vec::new(),
            };

        let engine =
            SettlementEngine::new();

        let result = engine
            .settle(
                &order,
                &execution,
            );

        assert!(matches!(
            result,
            Err(
                SettlementError
                    ::InvalidExecutionState
            )
        ));
    }
}