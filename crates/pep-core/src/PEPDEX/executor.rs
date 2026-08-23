use std::sync::Arc;
use std::thread;

use crate::PEPDEX::order::{
    ChildOrder,
    ExecutionPlan,
};
use crate::PEPDEX::venue::{
    Venue,
    VenueError,
    VenueId,
    VenueOrder,
    VenueOrderRequest,
    VenueOrderSide,
    VenueOrderType,
};

pub struct Executor {
    venues: Vec<Arc<dyn Venue>>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            venues: Vec::new(),
        }
    }

    pub fn add_venue<V>(&mut self, venue: V)
    where
        V: Venue + 'static,
    {
        self.venues.push(Arc::new(venue));
    }

    pub fn venue_count(&self) -> usize {
        self.venues.len()
    }

    pub fn execute(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<ExecutionResult, ExecutorError> {
        if plan.children.is_empty() {
            return Err(
                ExecutorError::EmptyExecutionPlan
            );
        }

        let mut handles = Vec::new();

        /*
         * Every child order is dispatched independently.
         *
         * Parent:
         *
         * BUY 100 PEP
         *
         * Children:
         *
         * Aster   20
         * Binance 40
         * EdgeX   35
         * OKX      5
         *
         * All four execution requests can therefore
         * be sent concurrently.
         */
        for child in &plan.children {
            let venue = self
                .find_venue(child.venue)?;

            let child = child.clone();

            let handle = thread::spawn(move || {
                execute_child(venue, child)
            });

            handles.push(handle);
        }

        let mut executions = Vec::new();
        let mut errors = Vec::new();

        for handle in handles {
            match handle.join() {
                Ok(Ok(execution)) => {
                    executions.push(execution);
                }

                Ok(Err(error)) => {
                    errors.push(error);
                }

                Err(_) => {
                    errors.push(
                        ExecutorError::WorkerPanicked
                    );
                }
            }
        }

        if executions.is_empty() && !errors.is_empty() {
            return Err(
                ExecutorError::AllChildrenFailed(errors)
            );
        }

        Ok(ExecutionResult {
            parent_order_id: plan.parent_order_id.clone(),
            executions,
            errors,
        })
    }

    fn find_venue(
        &self,
        venue_id: VenueId,
    ) -> Result<Arc<dyn Venue>, ExecutorError> {
        self.venues
            .iter()
            .find(|venue| venue.id() == venue_id)
            .cloned()
            .ok_or(
                ExecutorError::VenueNotConfigured(
                    venue_id
                )
            )
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

fn execute_child(
    venue: Arc<dyn Venue>,
    child: ChildOrder,
) -> Result<ChildExecution, ExecutorError> {
    let request = VenueOrderRequest {
        market: child.market.clone(),

        side: child.side,

        order_type: child.order_type,

        price: child.price,

        quantity: child.quantity,

        client_order_id:
            child.client_order_id.clone(),
    };

    let result = venue
        .place_order(&request)
        .map_err(|error| {
            ExecutorError::VenueExecutionFailed {
                venue: child.venue,
                child_order_id: child.id.clone(),
                error,
            }
        })?;

    Ok(ChildExecution {
        child_order: child,
        venue_order: result,
    })
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub parent_order_id:
        crate::PEPDEX::order::OrderId,

    pub executions: Vec<ChildExecution>,

    pub errors: Vec<ExecutorError>,
}

impl ExecutionResult {
    pub fn successful_count(&self) -> usize {
        self.executions.len()
    }

    pub fn failed_count(&self) -> usize {
        self.errors.len()
    }

    pub fn is_fully_dispatched(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug)]
pub struct ChildExecution {
    pub child_order: ChildOrder,
    pub venue_order: VenueOrder,
}

#[derive(Debug)]
pub enum ExecutorError {
    EmptyExecutionPlan,

    VenueNotConfigured(VenueId),

    VenueExecutionFailed {
        venue: VenueId,
        child_order_id:
            crate::PEPDEX::order::OrderId,
        error: VenueError,
    },

    WorkerPanicked,

    AllChildrenFailed(Vec<ExecutorError>),
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::order::{
        ChildOrder,
        ExecutionPlan,
        ParentOrder,
    };
    use crate::PEPDEX::venue::{
        aster::Aster,
        binance::Binance,
        edgeX::EdgeX,
        okx::Okx,
    };

    #[test]
    fn executor_registers_all_venues() {
        let mut executor = Executor::new();

        executor.add_venue(Aster::new());
        executor.add_venue(Binance::new());
        executor.add_venue(EdgeX::new());
        executor.add_venue(Okx::new());

        assert_eq!(
            executor.venue_count(),
            4
        );
    }

    #[test]
    fn executor_rejects_empty_plan() {
        let executor = Executor::new();

        let plan =
            ExecutionPlan::new(
                crate::PEPDEX::order::OrderId::new(
                    "PARENT-1"
                )
            );

        let result = executor.execute(&plan);

        assert!(matches!(
            result,
            Err(
                ExecutorError::EmptyExecutionPlan
            )
        ));
    }

    #[test]
    fn executor_rejects_unknown_venue() {
        let mut executor = Executor::new();

        executor.add_venue(Aster::new());

        let market =
            MarketId::new("PEP", "USDT");

        let parent = ParentOrder::new(
            "PARENT-1",
            "alice",
            market,
            VenueOrderSide::Buy,
            VenueOrderType::Limit,
            100,
            Some(602),
        );

        let child = ChildOrder::new(
            "CHILD-1",
            &parent,
            VenueId::Binance,
            100,
            Some(602),
        );

        let mut plan =
            ExecutionPlan::new(
                parent.id.clone()
            );

        plan.add_child(child);

        let result = executor.execute(&plan);

        assert!(matches!(
            result,
            Err(
                ExecutorError::VenueNotConfigured(
                    VenueId::Binance
                )
            )
        ));
    }
}