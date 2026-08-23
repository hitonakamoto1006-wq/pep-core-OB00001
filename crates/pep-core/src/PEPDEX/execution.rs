use crate::PEPDEX::order::{
    ChildOrder,
    ChildOrderStatus,
    OrderId,
};
use crate::PEPDEX::venue::{
    VenueId,
    VenueOrder,
    VenueOrderStatus,
};

#[derive(Debug, Clone)]
pub struct Execution {
    pub child_order_id: OrderId,
    pub parent_order_id: OrderId,

    pub venue: VenueId,

    pub requested_quantity: u64,
    pub filled_quantity: u64,
    pub remaining_quantity: u64,

    pub average_price: Option<u64>,

    /// Fee paid to the external venue.
    pub venue_fee: u64,

    pub status: ExecutionStatus,

    pub external_order_id: Option<String>,
}

impl Execution {
    pub fn from_child_order(
        child: &ChildOrder,
    ) -> Self {
        Self {
            child_order_id: child.id.clone(),
            parent_order_id: child.parent_id.clone(),

            venue: child.venue,

            requested_quantity: child.quantity,
            filled_quantity: 0,
            remaining_quantity: child.quantity,

            average_price: None,

            venue_fee: 0,

            status: ExecutionStatus::Pending,

            external_order_id: None,
        }
    }

    pub fn from_venue_order(
        child: &ChildOrder,
        venue_order: &VenueOrder,
    ) -> Self {
        let filled =
            venue_order.filled_quantity;

        let requested =
            venue_order.requested_quantity;

        let remaining =
            requested.saturating_sub(filled);

        Self {
            child_order_id: child.id.clone(),
            parent_order_id: child.parent_id.clone(),

            venue: venue_order.venue,

            requested_quantity: requested,
            filled_quantity: filled,
            remaining_quantity: remaining,

            average_price:
                venue_order.average_price,

            venue_fee: 0,

            status:
                ExecutionStatus::from_venue_status(
                    venue_order.status
                ),

            external_order_id:
                Some(
                    venue_order
                        .external_order_id
                        .clone()
                ),
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Filled
                | ExecutionStatus::Cancelled
                | ExecutionStatus::Rejected
        )
    }

    pub fn is_filled(&self) -> bool {
        self.status == ExecutionStatus::Filled
    }

    pub fn has_remaining(&self) -> bool {
        self.remaining_quantity > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    Pending,
    Submitted,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Failed,
}

impl ExecutionStatus {
    pub fn from_venue_status(
        status: VenueOrderStatus,
    ) -> Self {
        match status {
            VenueOrderStatus::New => {
                Self::Submitted
            }

            VenueOrderStatus::Open => {
                Self::Open
            }

            VenueOrderStatus::PartiallyFilled => {
                Self::PartiallyFilled
            }

            VenueOrderStatus::Filled => {
                Self::Filled
            }

            VenueOrderStatus::Cancelled => {
                Self::Cancelled
            }

            VenueOrderStatus::Rejected => {
                Self::Rejected
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionUpdate {
    pub child_order_id: OrderId,

    pub filled_quantity: u64,

    pub average_price: Option<u64>,

    pub venue_fee: u64,

    pub status: ExecutionStatus,

    pub external_order_id:
        Option<String>,
}

impl ExecutionUpdate {
    pub fn apply_to(
        &self,
        execution: &mut Execution,
    ) -> Result<(), ExecutionError> {
        if execution.child_order_id
            != self.child_order_id
        {
            return Err(
                ExecutionError::ChildOrderMismatch
            );
        }

        if self.filled_quantity
            > execution.requested_quantity
        {
            return Err(
                ExecutionError::FilledQuantityExceeded
            );
        }

        execution.filled_quantity =
            self.filled_quantity;

        execution.remaining_quantity =
            execution
                .requested_quantity
                .saturating_sub(
                    self.filled_quantity
                );

        execution.average_price =
            self.average_price;

        execution.venue_fee =
            self.venue_fee;

        execution.status =
            self.status;

        if self.external_order_id
            .is_some()
        {
            execution.external_order_id =
                self.external_order_id.clone();
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    ChildOrderMismatch,

    FilledQuantityExceeded,
}

#[derive(Debug, Default)]
pub struct ExecutionTracker {
    executions: Vec<Execution>,
}

impl ExecutionTracker {
    pub fn new() -> Self {
        Self {
            executions: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        execution: Execution,
    ) {
        self.executions.push(execution);
    }

    pub fn all(&self) -> &[Execution] {
        &self.executions
    }

    pub fn get(
        &self,
        child_order_id: &OrderId,
    ) -> Option<&Execution> {
        self.executions
            .iter()
            .find(|execution| {
                &execution.child_order_id
                    == child_order_id
            })
    }

    pub fn get_mut(
        &mut self,
        child_order_id: &OrderId,
    ) -> Option<&mut Execution> {
        self.executions
            .iter_mut()
            .find(|execution| {
                &execution.child_order_id
                    == child_order_id
            })
    }

    pub fn update(
        &mut self,
        update: ExecutionUpdate,
    ) -> Result<(), ExecutionError> {
        let execution = self
            .get_mut(&update.child_order_id)
            .ok_or(
                ExecutionError::ChildOrderMismatch
            )?;

        update.apply_to(execution)
    }

    pub fn filled_quantity(&self) -> u64 {
        self.executions
            .iter()
            .map(|execution| {
                execution.filled_quantity
            })
            .sum()
    }

    pub fn requested_quantity(&self) -> u64 {
        self.executions
            .iter()
            .map(|execution| {
                execution.requested_quantity
            })
            .sum()
    }

    pub fn remaining_quantity(&self) -> u64 {
        self.executions
            .iter()
            .map(|execution| {
                execution.remaining_quantity
            })
            .sum()
    }

    pub fn total_venue_fee(&self) -> u64 {
        self.executions
            .iter()
            .map(|execution| {
                execution.venue_fee
            })
            .sum()
    }

    pub fn is_fully_filled(&self) -> bool {
        self.requested_quantity() > 0
            && self.remaining_quantity() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::order::{
        ChildOrder,
        ParentOrder,
    };
    use crate::PEPDEX::venue::{
        VenueId,
        VenueOrderSide,
        VenueOrderStatus,
        VenueOrderType,
    };

    fn child_order() -> ChildOrder {
        let parent = ParentOrder::new(
            "PARENT-1",
            "alice",
            MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            VenueOrderType::Limit,
            100,
            Some(602),
        );

        ChildOrder::new(
            "CHILD-1",
            &parent,
            VenueId::Aster,
            100,
            Some(602),
        )
    }

    #[test]
    fn creates_pending_execution() {
        let child =
            child_order();

        let execution =
            Execution::from_child_order(
                &child
            );

        assert_eq!(
            execution.requested_quantity,
            100
        );

        assert_eq!(
            execution.filled_quantity,
            0
        );

        assert_eq!(
            execution.remaining_quantity,
            100
        );

        assert_eq!(
            execution.status,
            ExecutionStatus::Pending
        );
    }

    #[test]
    fn update_partial_fill() {
        let child =
            child_order();

        let mut execution =
            Execution::from_child_order(
                &child
            );

        let update =
            ExecutionUpdate {
                child_order_id:
                    child.id.clone(),

                filled_quantity: 40,

                average_price:
                    Some(602),

                venue_fee: 2,

                status:
                    ExecutionStatus::PartiallyFilled,

                external_order_id:
                    Some(
                        "ASTER-123"
                            .to_string()
                    ),
            };

        update
            .apply_to(&mut execution)
            .expect(
                "execution update failed"
            );

        assert_eq!(
            execution.filled_quantity,
            40
        );

        assert_eq!(
            execution.remaining_quantity,
            60
        );

        assert_eq!(
            execution.average_price,
            Some(602)
        );

        assert_eq!(
            execution.venue_fee,
            2
        );

        assert_eq!(
            execution.status,
            ExecutionStatus::PartiallyFilled
        );

        assert_eq!(
            execution.external_order_id
                .as_deref(),
            Some("ASTER-123")
        );
    }

    #[test]
    fn tracker_aggregates_fills() {
        let child =
            child_order();

        let mut tracker =
            ExecutionTracker::new();

        tracker.add(
            Execution {
                child_order_id:
                    child.id.clone(),

                parent_order_id:
                    child.parent_id.clone(),

                venue:
                    VenueId::Aster,

                requested_quantity:
                    100,

                filled_quantity:
                    60,

                remaining_quantity:
                    40,

                average_price:
                    Some(602),

                venue_fee:
                    2,

                status:
                    ExecutionStatus::PartiallyFilled,

                external_order_id:
                    Some(
                        "A-1".to_string()
                    ),
            }
        );

        assert_eq!(
            tracker.requested_quantity(),
            100
        );

        assert_eq!(
            tracker.filled_quantity(),
            60
        );

        assert_eq!(
            tracker.remaining_quantity(),
            40
        );

        assert_eq!(
            tracker.total_venue_fee(),
            2
        );
    }

    #[test]
    fn venue_status_mapping() {
        assert_eq!(
            ExecutionStatus::from_venue_status(
                VenueOrderStatus::New
            ),
            ExecutionStatus::Submitted
        );

        assert_eq!(
            ExecutionStatus::from_venue_status(
                VenueOrderStatus::PartiallyFilled
            ),
            ExecutionStatus::PartiallyFilled
        );

        assert_eq!(
            ExecutionStatus::from_venue_status(
                VenueOrderStatus::Filled
            ),
            ExecutionStatus::Filled
        );

        assert_eq!(
            ExecutionStatus::from_venue_status(
                VenueOrderStatus::Cancelled
            ),
            ExecutionStatus::Cancelled
        );
    }
}