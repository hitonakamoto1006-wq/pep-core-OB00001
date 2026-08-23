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
pub struct Binance {
    pub enabled: bool,
}

impl Binance {
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
            venue: VenueId::Binance,
            market: market.clone(),

            bids: vec![
                OrderBookLevel {
                    price: 600,
                    quantity: 20,
                },
                OrderBookLevel {
                    price: 599,
                    quantity: 80,
                },
                OrderBookLevel {
                    price: 598,
                    quantity: 120,
                },
            ],

            asks: vec![
                OrderBookLevel {
                    price: 602,
                    quantity: 40,
                },
                OrderBookLevel {
                    price: 604,
                    quantity: 60,
                },
                OrderBookLevel {
                    price: 605,
                    quantity: 100,
                },
            ],

            timestamp_ms: current_timestamp_ms(),
        }
    }
}

impl Default for Binance {
    fn default() -> Self {
        Self::new()
    }
}

impl Venue for Binance {
    fn id(&self) -> VenueId {
        VenueId::Binance
    }

    fn get_orderbook(
        &self,
        market: &MarketId,
    ) -> Result<VenueOrderBook, VenueError> {
        if !self.enabled {
            return Err(VenueError::Internal(
                "Binance adapter is disabled".to_string(),
            ));
        }

        /*
         * Temporary mock market data.
         *
         * This will later be replaced by Binance's
         * real WebSocket/local-orderbook implementation.
         */
        Ok(self.mock_orderbook(market))
    }

    fn place_order(
        &self,
        _request: &VenueOrderRequest,
    ) -> Result<VenueOrder, VenueError> {
        Err(VenueError::Unsupported(
            "Binance order execution is not connected yet"
                .to_string(),
        ))
    }

    fn cancel_order(
        &self,
        _external_order_id: &str,
    ) -> Result<(), VenueError> {
        Err(VenueError::Unsupported(
            "Binance order cancellation is not connected yet"
                .to_string(),
        ))
    }

    fn get_order(
        &self,
        _external_order_id: &str,
    ) -> Result<VenueOrder, VenueError> {
        Err(VenueError::Unsupported(
            "Binance order query is not connected yet"
                .to_string(),
        ))
    }
}

impl MarketDataVenue for Binance {
    fn id(&self) -> VenueId {
        VenueId::Binance
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