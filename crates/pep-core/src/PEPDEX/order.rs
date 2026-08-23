use crate::PEPDEX::market::MarketId;
use crate::PEPDEX::risk::OrderConstraints;
use crate::PEPDEX::venue::{
    VenueId,
    VenueOrderSide,
    VenueOrderType,
};

/// Unique identifier of a PEPDEX user order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(pub String);

impl OrderId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Order submitted by the PEPDEX user.
///
/// This is the parent order.
/// PEPDEX never sends this object directly to a venue.
///
/// ParentOrder
///     ↓
/// Router
///     ↓
/// ChildOrder[]
#[derive(Debug, Clone)]
pub struct ParentOrder {
    pub id: OrderId,

    pub user: String,

    pub market: MarketId,

    pub side: VenueOrderSide,

    pub order_type: VenueOrderType,

    /// Total quantity requested by the user.
    pub quantity: u64,

    /// Limit price.
///
/// None for market orders.
    pub limit_price: Option<u64>,

    /// Execution/risk constraints belonging to this order.
    pub constraints: OrderConstraints,

    pub timestamp_ms: u64,

    pub status: ParentOrderStatus,
}

impl ParentOrder {
    pub fn new(
        id: impl Into<String>,
        user: impl Into<String>,
        market: MarketId,
        side: VenueOrderSide,
        order_type: VenueOrderType,
        quantity: u64,
        limit_price: Option<u64>,
    ) -> Self {
        Self {
            id: OrderId::new(id),

            user: user.into(),

            market,

            side,

            order_type,

            quantity,

            limit_price,

            constraints:
                OrderConstraints::default(),

            timestamp_ms:
                current_timestamp_ms(),

            status:
                ParentOrderStatus::New,
        }
    }

    /// Attach risk constraints to the order.
    pub fn with_constraints(
        mut self,
        constraints: OrderConstraints,
    ) -> Self {
        self.constraints =
            constraints;

        self
    }

    pub fn is_market_order(&self) -> bool {
        self.order_type
            == VenueOrderType::Market
    }

    pub fn is_limit_order(&self) -> bool {
        self.order_type
            == VenueOrderType::Limit
    }

    pub fn max_spend(&self) -> Option<u64> {
        self.constraints.max_spend
    }

    pub fn max_slippage_bps(&self) -> Option<u64> {
        self.constraints.max_slippage_bps
    }

    pub fn max_quantity(&self) -> Option<u64> {
        self.constraints.max_quantity
    }
}

/// State of the parent order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentOrderStatus {
    New,
    Routing,
    PartiallyFilled,
    Filled,
    Cancelled,
    Failed,
}

/// Child order created by PEPDEX.
///
/// One parent can produce many children:
///
/// Parent:
///     BUY 100 PEP
///
/// Children:
///     Aster   20
///     Binance 40
///     EdgeX   35
///     OKX      5
#[derive(Debug, Clone)]
pub struct ChildOrder {
    pub id: OrderId,

    pub parent_id: OrderId,

    pub venue: VenueId,

    pub market: MarketId,

    pub side: VenueOrderSide,

    pub order_type: VenueOrderType,

    pub quantity: u64,

    pub price: Option<u64>,

    pub client_order_id: String,

    pub timestamp_ms: u64,

    pub status: ChildOrderStatus,
}

impl ChildOrder {
    pub fn new(
        id: impl Into<String>,
        parent: &ParentOrder,
        venue: VenueId,
        quantity: u64,
        price: Option<u64>,
    ) -> Self {
        let id = OrderId::new(id);

        Self {
            client_order_id:
                format!(
                    "PEPDEX-{}",
                    id.as_str()
                ),

            id,

            parent_id:
                parent.id.clone(),

            venue,

            market:
                parent.market.clone(),

            side:
                parent.side,

            order_type:
                parent.order_type,

            quantity,

            price,

            timestamp_ms:
                current_timestamp_ms(),

            status:
                ChildOrderStatus::Pending,
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            ChildOrderStatus::Filled
                | ChildOrderStatus::Cancelled
                | ChildOrderStatus::Rejected
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildOrderStatus {
    Pending,
    Submitted,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// Result of routing a parent order.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub parent_order_id: OrderId,

    pub children: Vec<ChildOrder>,
}

impl ExecutionPlan {
    pub fn new(
        parent_order_id: OrderId,
    ) -> Self {
        Self {
            parent_order_id,

            children:
                Vec::new(),
        }
    }

    pub fn add_child(
        &mut self,
        child: ChildOrder,
    ) {
        self.children.push(child);
    }

    pub fn total_quantity(&self) -> u64 {
        self.children
            .iter()
            .map(|child| {
                child.quantity
            })
            .sum()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

fn current_timestamp_ms() -> u64 {
    use std::time::{
        SystemTime,
        UNIX_EPOCH,
    };

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::risk::OrderConstraints;
    use crate::PEPDEX::venue::{
        VenueOrderSide,
        VenueOrderType,
    };

    fn market() -> MarketId {
        MarketId::new(
            "PEP",
            "USDT",
        )
    }

    #[test]
    fn creates_parent_order() {
        let order =
            ParentOrder::new(
                "ORDER-1",
                "alice",
                market(),
                VenueOrderSide::Buy,
                VenueOrderType::Limit,
                100,
                Some(602),
            );

        assert_eq!(
            order.id.as_str(),
            "ORDER-1"
        );

        assert_eq!(
            order.user,
            "alice"
        );

        assert_eq!(
            order.quantity,
            100
        );

        assert_eq!(
            order.limit_price,
            Some(602)
        );

        assert!(
            order.is_limit_order()
        );

        assert!(
            !order.is_market_order()
        );
    }

    #[test]
    fn constraints_belong_to_parent_order() {
        let constraints =
            OrderConstraints::new()
                .with_max_spend(
                    70_000
                )
                .with_max_slippage_bps(
                    100
                );

        let order =
            ParentOrder::new(
                "ORDER-2",
                "alice",
                market(),
                VenueOrderSide::Buy,
                VenueOrderType::Market,
                100,
                None,
            )
            .with_constraints(
                constraints
            );

        assert_eq!(
            order.max_spend(),
            Some(70_000)
        );

        assert_eq!(
            order.max_slippage_bps(),
            Some(100)
        );
    }

    #[test]
    fn child_inherits_parent_properties() {
        let parent =
            ParentOrder::new(
                "ORDER-3",
                "alice",
                market(),
                VenueOrderSide::Buy,
                VenueOrderType::Limit,
                100,
                Some(602),
            );

        let child =
            ChildOrder::new(
                "CHILD-1",
                &parent,
                VenueId::Aster,
                40,
                Some(602),
            );

        assert_eq!(
            child.parent_id,
            parent.id
        );

        assert_eq!(
            child.market,
            parent.market
        );

        assert_eq!(
            child.side,
            parent.side
        );

        assert_eq!(
            child.quantity,
            40
        );
    }

    #[test]
    fn execution_plan_tracks_children() {
        let parent =
            ParentOrder::new(
                "ORDER-4",
                "alice",
                market(),
                VenueOrderSide::Buy,
                VenueOrderType::Market,
                100,
                None,
            );

        let child_a =
            ChildOrder::new(
                "CHILD-A",
                &parent,
                VenueId::Aster,
                60,
                None,
            );

        let child_b =
            ChildOrder::new(
                "CHILD-B",
                &parent,
                VenueId::Binance,
                40,
                None,
            );

        let mut plan =
            ExecutionPlan::new(
                parent.id.clone()
            );

        plan.add_child(
            child_a
        );

        plan.add_child(
            child_b
        );

        assert_eq!(
            plan.child_count(),
            2
        );

        assert_eq!(
            plan.total_quantity(),
            100
        );
    }
}