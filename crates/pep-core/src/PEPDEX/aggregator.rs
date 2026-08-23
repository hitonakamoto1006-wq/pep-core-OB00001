use crate::PEPDEX::execution::{
    Execution,
    ExecutionStatus,
};
use crate::PEPDEX::order::{
    OrderId,
    ParentOrder,
};
use crate::PEPDEX::venue::VenueId;

#[derive(Debug, Clone)]
pub struct ParentExecution {
    pub parent_order_id: OrderId,

    pub requested_quantity: u64,
    pub filled_quantity: u64,
    pub remaining_quantity: u64,

    pub total_cost: u64,
    pub total_venue_fee: u64,

    pub average_price: Option<u64>,

    pub status: ParentExecutionStatus,

    pub venue_executions: Vec<VenueExecutionSummary>,
}

impl ParentExecution {
    pub fn new(
        parent_order: &ParentOrder,
    ) -> Self {
        Self {
            parent_order_id: parent_order.id.clone(),

            requested_quantity:
                parent_order.quantity,

            filled_quantity: 0,
            remaining_quantity:
                parent_order.quantity,

            total_cost: 0,
            total_venue_fee: 0,

            average_price: None,

            status:
                ParentExecutionStatus::Pending,

            venue_executions: Vec::new(),
        }
    }

    pub fn is_filled(&self) -> bool {
        self.status
            == ParentExecutionStatus::Filled
    }

    pub fn is_partial(&self) -> bool {
        self.status
            == ParentExecutionStatus::PartiallyFilled
    }

    pub fn has_remaining(&self) -> bool {
        self.remaining_quantity > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentExecutionStatus {
    Pending,
    Executing,
    PartiallyFilled,
    Filled,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone)]
pub struct VenueExecutionSummary {
    pub venue: VenueId,

    pub requested_quantity: u64,
    pub filled_quantity: u64,
    pub remaining_quantity: u64,

    pub average_price: Option<u64>,
    pub venue_fee: u64,

    pub execution_count: usize,
}

pub struct ExecutionAggregator;

impl ExecutionAggregator {
    pub fn new() -> Self {
        Self
    }

    pub fn aggregate(
        &self,
        parent_order: &ParentOrder,
        executions: &[Execution],
    ) -> Result<ParentExecution, AggregatorError> {
        if executions.is_empty() {
            return Err(
                AggregatorError::NoExecutions
            );
        }

        for execution in executions {
            if execution.parent_order_id
                != parent_order.id
            {
                return Err(
                    AggregatorError::ParentOrderMismatch
                );
            }
        }

        let mut result =
            ParentExecution::new(parent_order);

        /*
         * First aggregate raw execution quantities.
         */
        result.requested_quantity = executions
            .iter()
            .map(|execution| {
                execution.requested_quantity
            })
            .sum();

        result.filled_quantity = executions
            .iter()
            .map(|execution| {
                execution.filled_quantity
            })
            .sum();

        result.remaining_quantity =
            result
                .requested_quantity
                .saturating_sub(
                    result.filled_quantity
                );

        result.total_venue_fee = executions
            .iter()
            .map(|execution| {
                execution.venue_fee
            })
            .sum();

        /*
         * Calculate total execution value.
         *
         * quantity × average price
         *
         * All quantities/prices are represented using
         * PEPDEX fixed-point units.
         */
        result.total_cost = executions
            .iter()
            .map(|execution| {
                match execution.average_price {
                    Some(price) => execution
                        .filled_quantity
                        .saturating_mul(price),

                    None => 0,
                }
            })
            .sum();

        /*
         * Weighted average execution price.
         */
        if result.filled_quantity > 0 {
            result.average_price =
                Some(
                    result.total_cost
                        / result.filled_quantity
                );
        }

        /*
         * Summarize execution by venue.
         */
        result.venue_executions =
            Self::summarize_venues(
                executions
            );

        /*
         * Determine parent execution state.
         */
        result.status =
            Self::derive_status(
                &result,
                executions
            );

        /*
         * The parent requested quantity must match
         * the order quantity.
         *
         * This catches accidental execution records
         * belonging to the wrong parent.
         */
        if result.requested_quantity
            != parent_order.quantity
        {
            return Err(
                AggregatorError::RequestedQuantityMismatch {
                    order_quantity:
                        parent_order.quantity,

                    execution_quantity:
                        result.requested_quantity,
                }
            );
        }

        Ok(result)
    }

    fn summarize_venues(
        executions: &[Execution],
    ) -> Vec<VenueExecutionSummary> {
        let mut summaries: Vec<
            VenueExecutionSummary
        > = Vec::new();

        for execution in executions {
            if let Some(summary) =
                summaries.iter_mut().find(
                    |summary| {
                        summary.venue
                            == execution.venue
                    }
                )
            {
                summary.requested_quantity +=
                    execution.requested_quantity;

                summary.filled_quantity +=
                    execution.filled_quantity;

                summary.remaining_quantity +=
                    execution.remaining_quantity;

                summary.venue_fee +=
                    execution.venue_fee;

                summary.execution_count += 1;

                /*
                 * Recalculate weighted average price
                 * for this venue.
                 */
                let old_value =
                    summary
                        .average_price
                        .unwrap_or(0)
                        .saturating_mul(
                            summary
                                .filled_quantity
                                .saturating_sub(
                                    execution.filled_quantity
                                )
                        );

                let new_value =
                    execution
                        .average_price
                        .unwrap_or(0)
                        .saturating_mul(
                            execution
                                .filled_quantity
                        );

                if summary.filled_quantity > 0 {
                    summary.average_price =
                        Some(
                            old_value
                                .saturating_add(
                                    new_value
                                )
                                / summary
                                    .filled_quantity
                        );
                }
            } else {
                summaries.push(
                    VenueExecutionSummary {
                        venue:
                            execution.venue,

                        requested_quantity:
                            execution
                                .requested_quantity,

                        filled_quantity:
                            execution
                                .filled_quantity,

                        remaining_quantity:
                            execution
                                .remaining_quantity,

                        average_price:
                            execution
                                .average_price,

                        venue_fee:
                            execution.venue_fee,

                        execution_count: 1,
                    }
                );
            }
        }

        summaries
    }

    fn derive_status(
        result: &ParentExecution,
        executions: &[Execution],
    ) -> ParentExecutionStatus {
        if result.filled_quantity
            == result.requested_quantity
        {
            return ParentExecutionStatus::Filled;
        }

        if result.filled_quantity > 0 {
            return ParentExecutionStatus::PartiallyFilled;
        }

        if executions.iter().all(|execution| {
            execution.status
                == ExecutionStatus::Rejected
                || execution.status
                    == ExecutionStatus::Failed
        }) {
            return ParentExecutionStatus::Failed;
        }

        if executions.iter().all(|execution| {
            execution.status
                == ExecutionStatus::Cancelled
        }) {
            return ParentExecutionStatus::Cancelled;
        }

        ParentExecutionStatus::Executing
    }
}

impl Default for ExecutionAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum AggregatorError {
    NoExecutions,

    ParentOrderMismatch,

    RequestedQuantityMismatch {
        order_quantity: u64,
        execution_quantity: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::execution::{
        Execution,
        ExecutionStatus,
    };
    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::order::ParentOrder;
    use crate::PEPDEX::venue::{
        VenueId,
        VenueOrderSide,
        VenueOrderType,
    };

    fn parent() -> ParentOrder {
        ParentOrder::new(
            "PARENT-1",
            "alice",
            MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            VenueOrderType::Market,
            100,
            None,
        )
    }

    #[test]
    fn aggregates_multiple_venues() {
        let parent = parent();

        let executions = vec![
            Execution {
                child_order_id:
                    OrderId::new("CHILD-A"),

                parent_order_id:
                    parent.id.clone(),

                venue:
                    VenueId::Aster,

                requested_quantity: 20,
                filled_quantity: 20,
                remaining_quantity: 0,

                average_price:
                    Some(602),

                venue_fee: 1,

                status:
                    ExecutionStatus::Filled,

                external_order_id:
                    Some("A-1".into()),
            },

            Execution {
                child_order_id:
                    OrderId::new("CHILD-B"),

                parent_order_id:
                    parent.id.clone(),

                venue:
                    VenueId::Binance,

                requested_quantity: 40,
                filled_quantity: 40,
                remaining_quantity: 0,

                average_price:
                    Some(602),

                venue_fee: 2,

                status:
                    ExecutionStatus::Filled,

                external_order_id:
                    Some("B-1".into()),
            },

            Execution {
                child_order_id:
                    OrderId::new("CHILD-C"),

                parent_order_id:
                    parent.id.clone(),

                venue:
                    VenueId::EdgeX,

                requested_quantity: 35,
                filled_quantity: 35,
                remaining_quantity: 0,

                average_price:
                    Some(603),

                venue_fee: 2,

                status:
                    ExecutionStatus::Filled,

                external_order_id:
                    Some("E-1".into()),
            },

            Execution {
                child_order_id:
                    OrderId::new("CHILD-D"),

                parent_order_id:
                    parent.id.clone(),

                venue:
                    VenueId::Okx,

                requested_quantity: 5,
                filled_quantity: 5,
                remaining_quantity: 0,

                average_price:
                    Some(604),

                venue_fee: 1,

                status:
                    ExecutionStatus::Filled,

                external_order_id:
                    Some("O-1".into()),
            },
        ];

        let aggregator =
            ExecutionAggregator::new();

        let result = aggregator
            .aggregate(
                &parent,
                &executions
            )
            .expect(
                "aggregation failed"
            );

        assert_eq!(
            result.requested_quantity,
            100
        );

        assert_eq!(
            result.filled_quantity,
            100
        );

        assert_eq!(
            result.remaining_quantity,
            0
        );

        assert_eq!(
            result.total_venue_fee,
            6
        );

        /*
         * Total cost:
         *
         * 20 × 602 = 12040
         * 40 × 602 = 24080
         * 35 × 603 = 21105
         *  5 × 604 =  3020
         *
         * total = 60245
         *
         * average = 602
         */
        assert_eq!(
            result.total_cost,
            60245
        );

        assert_eq!(
            result.average_price,
            Some(602)
        );

        assert_eq!(
            result.status,
            ParentExecutionStatus::Filled
        );

        assert_eq!(
            result.venue_executions.len(),
            4
        );
    }

    #[test]
    fn detects_partial_execution() {
        let parent = parent();

        let executions = vec![
            Execution {
                child_order_id:
                    OrderId::new("CHILD-A"),

                parent_order_id:
                    parent.id.clone(),

                venue:
                    VenueId::Aster,

                requested_quantity: 60,
                filled_quantity: 60,
                remaining_quantity: 0,

                average_price:
                    Some(602),

                venue_fee: 1,

                status:
                    ExecutionStatus::Filled,

                external_order_id:
                    Some("A-1".into()),
            },

            Execution {
                child_order_id:
                    OrderId::new("CHILD-B"),

                parent_order_id:
                    parent.id.clone(),

                venue:
                    VenueId::Binance,

                requested_quantity: 40,
                filled_quantity: 20,
                remaining_quantity: 20,

                average_price:
                    Some(603),

                venue_fee: 1,

                status:
                    ExecutionStatus::PartiallyFilled,

                external_order_id:
                    Some("B-1".into()),
            },

        ];

        let aggregator =
            ExecutionAggregator::new();

        let result = aggregator
            .aggregate(
                &parent,
                &executions
            )
            .expect(
                "aggregation failed"
            );

        assert_eq!(
            result.filled_quantity,
            80
        );

        assert_eq!(
            result.remaining_quantity,
            20
        );

        assert_eq!(
            result.status,
            ParentExecutionStatus::PartiallyFilled
        );
    }

    #[test]
    fn rejects_wrong_parent() {
        let parent = parent();

        let other_parent =
            ParentOrder::new(
                "OTHER",
                "bob",
                MarketId::new(
                    "PEP",
                    "USDT",
                ),
                VenueOrderSide::Buy,
                VenueOrderType::Market,
                100,
                None,
            );

        let execution = Execution {
            child_order_id:
                OrderId::new("CHILD-X"),

            parent_order_id:
                other_parent.id.clone(),

            venue:
                VenueId::Aster,

            requested_quantity: 100,
            filled_quantity: 100,
            remaining_quantity: 0,

            average_price:
                Some(602),

            venue_fee: 1,

            status:
                ExecutionStatus::Filled,

            external_order_id:
                Some("X-1".into()),
        };

        let aggregator =
            ExecutionAggregator::new();

        let result = aggregator.aggregate(
            &parent,
            &[execution]
        );

        assert!(matches!(
            result,
            Err(
                AggregatorError::ParentOrderMismatch
            )
        ));
    }
}