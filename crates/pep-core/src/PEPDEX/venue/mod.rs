use crate::PEPDEX::market::MarketId;

pub mod aster;
pub mod binance;
pub mod edgeX;
pub mod okx;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Ord,
    PartialOrd,
)]
pub enum VenueId {
    Aster,
    Binance,
    EdgeX,
    Okx,
}

impl VenueId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aster => "aster",
            Self::Binance => "binance",
            Self::EdgeX => "edgex",
            Self::Okx => "okx",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueOrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueOrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueOrderStatus {
    New,
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// A single price/quantity level in fixed-point representation.
///
/// `price` and `quantity` are stored as integers.
///
/// Their decimal interpretation is defined by the associated
/// `Market`:
///
/// ```text
/// price    -> market.price_scale
/// quantity -> market.quantity_scale
/// ```
///
/// Example:
///
/// ```text
/// BTC/USDT
/// price_scale = 2
/// quantity_scale = 6
///
/// API:
///     price    = "65190.25"
///     quantity = "0.142"
///
/// Stored:
///     price    = 6_519_025
///     quantity = 142_000
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderBookLevel {
    pub price: u64,
    pub quantity: u64,
}

impl OrderBookLevel {
    pub fn new(
        price: u64,
        quantity: u64,
    ) -> Self {
        Self {
            price,
            quantity,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.quantity == 0
    }
}

/// Orderbook returned by an individual venue.
#[derive(Debug, Clone)]
pub struct VenueOrderBook {
    pub venue: VenueId,
    pub market: MarketId,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct VenueOrderRequest {
    pub market: MarketId,
    pub side: VenueOrderSide,
    pub order_type: VenueOrderType,

    /// Fixed-point price.
    pub price: Option<u64>,

    /// Fixed-point quantity.
    pub quantity: u64,

    pub client_order_id: String,
}

#[derive(Debug, Clone)]
pub struct VenueOrder {
    pub venue: VenueId,
    pub external_order_id: String,
    pub client_order_id: String,
    pub market: MarketId,
    pub side: VenueOrderSide,

    /// Fixed-point quantity.
    pub requested_quantity: u64,

    /// Fixed-point quantity.
    pub filled_quantity: u64,

    /// Fixed-point price.
    pub average_price: Option<u64>,

    pub status: VenueOrderStatus,
}

#[derive(Debug)]
pub enum VenueError {
    Network(String),
    Authentication(String),
    InvalidRequest(String),
    NotFound(String),
    RateLimited,
    Unsupported(String),
    Internal(String),
}

/// Generic venue interface.
///
/// Execution and market-data adapters can implement this
/// independently.
pub trait Venue: Send + Sync {
    fn id(&self) -> VenueId;

    fn get_orderbook(
        &self,
        market: &MarketId,
    ) -> Result<VenueOrderBook, VenueError>;

    fn place_order(
        &self,
        request: &VenueOrderRequest,
    ) -> Result<VenueOrder, VenueError>;

    fn cancel_order(
        &self,
        external_order_id: &str,
    ) -> Result<(), VenueError>;

    fn get_order(
        &self,
        external_order_id: &str,
    ) -> Result<VenueOrder, VenueError>;
}

/// Market-data source.
///
/// This is deliberately separated from `Venue` so the market-data
/// subsystem can later run persistent WebSocket streams independently
/// from order execution.
pub trait MarketDataVenue: Send + Sync {
    fn id(&self) -> VenueId;

    fn snapshot(
        &self,
        market: &MarketId,
    ) -> Result<VenueOrderBook, VenueError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PEPDEX::market::Market;

    #[test]
    fn orderbook_level_keeps_fixed_point_values() {
        let level =
            OrderBookLevel::new(
                6_519_025,
                142_000,
            );

        assert_eq!(
            level.price,
            6_519_025
        );

        assert_eq!(
            level.quantity,
            142_000
        );

        assert!(!level.is_empty());
    }

    #[test]
    fn empty_orderbook_level_is_detected() {
        let level =
            OrderBookLevel::new(
                6_519_025,
                0,
            );

        assert!(level.is_empty());
    }

    #[test]
    fn market_precision_defines_interpretation() {
        let market =
            Market::with_precision(
                "BTC",
                "USDT",
                2,
                6,
            );

        assert_eq!(
            market.price_multiplier(),
            100
        );

        assert_eq!(
            market.quantity_multiplier(),
            1_000_000
        );

        let level =
            OrderBookLevel::new(
                6_519_025,
                142_000,
            );

        assert_eq!(
            level.price / market.price_multiplier(),
            65_190
        );

        assert_eq!(
            level.quantity,
            142_000
        );
    }
}