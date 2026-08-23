use crate::PEPDEX::market::MarketId;
use crate::PEPDEX::venue::{
    MarketDataVenue,
    OrderBookLevel,
    Venue,
    VenueError,
    VenueId,
    VenueOrder,
    VenueOrderBook,
    VenueOrderRequest,
};

#[derive(Debug, Clone)]
pub struct EdgeX {
    pub enabled: bool,
}

impl EdgeX {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    fn mock_orderbook(
        &self,
        market: &MarketId,
    ) -> VenueOrderBook {
        VenueOrderBook {
            venue: VenueId::EdgeX,
            market: market.clone(),

            bids: vec![
                OrderBookLevel {
                    price: 599,
                    quantity: 25,
                },
                OrderBookLevel {
                    price: 598,
                    quantity: 45,
                },
                OrderBookLevel {
                    price: 597,
                    quantity: 70,
                },
            ],

            asks: vec![
                OrderBookLevel {
                    price: 603,
                    quantity: 35,
                },
                OrderBookLevel {
                    price: 605,
                    quantity: 50,
                },
                OrderBookLevel {
                    price: 606,
                    quantity: 80,
                },
            ],

            timestamp_ms: current_timestamp_ms(),
        }
    }
}

impl Default for EdgeX {
    fn default() -> Self {
        Self::new()
    }
}

impl Venue for EdgeX {
    fn id(&self) -> VenueId {
        VenueId::EdgeX
    }

    fn get_orderbook(
        &self,
        market: &MarketId,
    ) -> Result<VenueOrderBook, VenueError> {
        if !self.enabled {
            return Err(VenueError::Internal(
                "EdgeX adapter is disabled".to_string(),
            ));
        }

        /*
         * Temporary mock market data.
         *
         * This will later be replaced by EdgeX's
         * real WebSocket/local-orderbook implementation.
         */
        Ok(self.mock_orderbook(market))
    }

    fn place_order(
        &self,
        _request: &VenueOrderRequest,
    ) -> Result<VenueOrder, VenueError> {
        Err(VenueError::Unsupported(
            "EdgeX order execution is not connected yet"
                .to_string(),
        ))
    }

    fn cancel_order(
        &self,
        _external_order_id: &str,
    ) -> Result<(), VenueError> {
        Err(VenueError::Unsupported(
            "EdgeX order cancellation is not connected yet"
                .to_string(),
        ))
    }

    fn get_order(
        &self,
        _external_order_id: &str,
    ) -> Result<VenueOrder, VenueError> {
        Err(VenueError::Unsupported(
            "EdgeX order query is not connected yet"
                .to_string(),
        ))
    }
}

impl MarketDataVenue for EdgeX {
    fn id(&self) -> VenueId {
        VenueId::EdgeX
    }

    fn snapshot(
        &self,
        market: &MarketId,
    ) -> Result<VenueOrderBook, VenueError> {
        self.get_orderbook(market)
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