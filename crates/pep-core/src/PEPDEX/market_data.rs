use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;

use crate::PEPDEX::market::MarketId;
use crate::PEPDEX::orderbook::{
    aggregate_orderbooks,
    AggregatedOrderBook,
};
use crate::PEPDEX::venue::{
    MarketDataVenue,
    VenueError,
    VenueId,
    VenueOrderBook,
};

#[derive(Clone)]
pub struct LocalOrderBookStore {
    books: Arc<RwLock<HashMap<VenueId, VenueOrderBook>>>,
}

impl LocalOrderBookStore {
    pub fn new() -> Self {
        Self {
            books: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn update(&self, book: VenueOrderBook) {
        let mut books = self
            .books
            .write()
            .expect("local orderbook store poisoned");

        books.insert(book.venue, book);
    }

    pub fn get(
        &self,
        venue: VenueId,
    ) -> Option<VenueOrderBook> {
        let books = self
            .books
            .read()
            .expect("local orderbook store poisoned");

        books.get(&venue).cloned()
    }

    pub fn all(&self) -> Vec<VenueOrderBook> {
        let books = self
            .books
            .read()
            .expect("local orderbook store poisoned");

        books.values().cloned().collect()
    }

    pub fn len(&self) -> usize {
        let books = self
            .books
            .read()
            .expect("local orderbook store poisoned");

        books.len()
    }

    pub fn clear(&self) {
        let mut books = self
            .books
            .write()
            .expect("local orderbook store poisoned");

        books.clear();
    }
}

impl Default for LocalOrderBookStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MarketDataEngine {
    venues: Vec<Arc<dyn MarketDataVenue>>,
    store: LocalOrderBookStore,
}

impl MarketDataEngine {
    pub fn new() -> Self {
        Self {
            venues: Vec::new(),
            store: LocalOrderBookStore::new(),
        }
    }

    pub fn add_venue<V>(&mut self, venue: V)
    where
        V: MarketDataVenue + 'static,
    {
        self.venues.push(Arc::new(venue));
    }

    pub fn venue_count(&self) -> usize {
        self.venues.len()
    }

    pub fn store(&self) -> LocalOrderBookStore {
        self.store.clone()
    }

    pub fn refresh(
        &self,
        market: &MarketId,
    ) -> Result<usize, VenueError> {
        if self.venues.is_empty() {
            return Err(VenueError::Internal(
                "No market-data venues configured".to_string(),
            ));
        }

        let mut handles =
            Vec::with_capacity(self.venues.len());

        /*
         * Fan-out:
         *
         * Mỗi venue được chạy trong một worker riêng.
         *
         * Aster
         * Binance
         * EdgeX
         * OKX
         *
         * có thể lấy dữ liệu đồng thời.
         */
        for venue in &self.venues {
            let venue = Arc::clone(venue);
            let store = self.store.clone();
            let market = market.clone();

            let handle = thread::spawn(move || {
                let venue_id = venue.id();

                match venue.snapshot(&market) {
                    Ok(book) => {
                        store.update(book);
                        Ok(venue_id)
                    }

                    Err(error) => {
                        Err((venue_id, error))
                    }
                }
            });

            handles.push(handle);
        }

        /*
         * Fan-in:
         *
         * Chờ tất cả worker hoàn thành.
         */
        let mut successful = 0usize;
        let mut errors = Vec::new();

        for handle in handles {
            match handle.join() {
                Ok(Ok(_venue)) => {
                    successful += 1;
                }

                Ok(Err((venue, error))) => {
                    errors.push(format!(
                        "{}: {:?}",
                        venue.as_str(),
                        error
                    ));
                }

                Err(_) => {
                    errors.push(
                        "market-data worker panicked"
                            .to_string(),
                    );
                }
            }
        }

        /*
         * Nếu toàn bộ venue đều fail thì refresh fail.
         *
         * Nếu chỉ một venue fail thì vẫn giữ các
         * local orderbook thành công.
         */
        if successful == 0 {
            return Err(VenueError::Network(format!(
                "All market-data venues failed: {}",
                errors.join("; ")
            )));
        }

        Ok(successful)
    }

    pub fn aggregated(
        &self,
    ) -> Option<AggregatedOrderBook> {
        let books = self.store.all();

        if books.is_empty() {
            return None;
        }

        aggregate_orderbooks(&books)
    }

    pub fn snapshot(
        &self,
        market: &MarketId,
    ) -> Result<AggregatedOrderBook, VenueError> {
        self.refresh(market)?;

        self.aggregated().ok_or_else(|| {
            VenueError::Internal(
                "No local orderbooks available".to_string(),
            )
        })
    }

    pub fn local_books(
        &self,
    ) -> Vec<VenueOrderBook> {
        self.store.all()
    }
}

impl Default for MarketDataEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::venue::{
        MarketDataVenue,
        OrderBookLevel,
        VenueError,
        VenueId,
        VenueOrderBook,
    };

    /*
     * =========================================================
     * DETERMINISTIC MOCK VENUE
     * =========================================================
     *
     * Test không gọi API thật.
     *
     * Vì vậy:
     *
     * - không cần Internet
     * - không phụ thuộc API exchange
     * - không phụ thuộc latency
     * - không phụ thuộc market data hiện tại
     * - test luôn deterministic
     *
     * Production adapter Aster/Binance/EdgeX/OKX
     * vẫn hoàn toàn độc lập với mock này.
     */

    #[derive(Clone)]
    struct MockVenue {
        id: VenueId,

        bid_price: u64,
        bid_quantity: u64,

        ask_price: u64,
        ask_quantity: u64,
    }

    impl MockVenue {
        fn new(
            id: VenueId,
            bid_price: u64,
            bid_quantity: u64,
            ask_price: u64,
            ask_quantity: u64,
        ) -> Self {
            Self {
                id,
                bid_price,
                bid_quantity,
                ask_price,
                ask_quantity,
            }
        }

        fn orderbook(
            &self,
            market: &MarketId,
        ) -> VenueOrderBook {
            VenueOrderBook {
                venue: self.id,

                market: market.clone(),

                bids: vec![
                    OrderBookLevel {
                        price: self.bid_price,
                        quantity: self.bid_quantity,
                    },
                ],

                asks: vec![
                    OrderBookLevel {
                        price: self.ask_price,
                        quantity: self.ask_quantity,
                    },
                ],

                timestamp_ms: 0,
            }
        }
    }

    impl MarketDataVenue for MockVenue {
        fn id(&self) -> VenueId {
            self.id
        }

        fn snapshot(
            &self,
            market: &MarketId,
        ) -> Result<VenueOrderBook, VenueError> {
            Ok(self.orderbook(market))
        }
    }

    /*
     * =========================================================
     * MOCK VENUES
     * =========================================================
     *
     * Aster:
     *     BID 600 × 30
     *     ASK 602 × 20
     *
     * Binance:
     *     BID 600 × 20
     *     ASK 602 × 40
     *
     * EdgeX:
     *     BID 599 × 50
     *     ASK 603 × 35
     *
     * OKX:
     *     BID 598 × 75
     *     ASK 604 × 65
     */

    fn mock_aster() -> MockVenue {
        MockVenue::new(
            VenueId::Aster,
            600,
            30,
            602,
            20,
        )
    }

    fn mock_binance() -> MockVenue {
        MockVenue::new(
            VenueId::Binance,
            600,
            20,
            602,
            40,
        )
    }

    fn mock_edgex() -> MockVenue {
        MockVenue::new(
            VenueId::EdgeX,
            599,
            50,
            603,
            35,
        )
    }

    fn mock_okx() -> MockVenue {
        MockVenue::new(
            VenueId::Okx,
            598,
            75,
            604,
            65,
        )
    }

    fn engine_with_four_mock_venues()
        -> MarketDataEngine
    {
        let mut engine =
            MarketDataEngine::new();

        engine.add_venue(mock_aster());
        engine.add_venue(mock_binance());
        engine.add_venue(mock_edgex());
        engine.add_venue(mock_okx());

        engine
    }

    #[test]
    fn four_venues_run_in_parallel() {
        let engine =
            engine_with_four_mock_venues();

        assert_eq!(
            engine.venue_count(),
            4
        );

        let market =
            MarketId::new("PEP", "USDT");

        let successful = engine
            .refresh(&market)
            .expect(
                "market-data refresh failed"
            );

        assert_eq!(
            successful,
            4
        );

        assert_eq!(
            engine.store().len(),
            4
        );
    }

    #[test]
    fn aggregates_all_four_venues() {
        let engine =
            engine_with_four_mock_venues();

        let market =
            MarketId::new("PEP", "USDT");

        let book = engine
            .snapshot(&market)
            .expect("snapshot failed");

        assert_eq!(
            book.market,
            market
        );

        /*
         * Best bid:
         *
         * Aster:
         *     600 × 30
         *
         * Binance:
         *     600 × 20
         *
         * EdgeX:
         *     599 × 50
         *
         * OKX:
         *     598 × 75
         *
         * => 600 × 50
         */

        let best_bid = book
            .best_bid()
            .expect(
                "missing best bid"
            );

        assert_eq!(
            best_bid.price,
            600
        );

        assert_eq!(
            best_bid.quantity,
            50
        );

        /*
         * Best ask:
         *
         * Aster:
         *     602 × 20
         *
         * Binance:
         *     602 × 40
         *
         * EdgeX:
         *     603 × 35
         *
         * OKX:
         *     604 × 65
         *
         * => 602 × 60
         */

        let best_ask = book
            .best_ask()
            .expect(
                "missing best ask"
            );

        assert_eq!(
            best_ask.price,
            602
        );

        assert_eq!(
            best_ask.quantity,
            60
        );
    }

    #[test]
    fn keeps_each_venue_local_book() {
        let engine =
            engine_with_four_mock_venues();

        let market =
            MarketId::new("PEP", "USDT");

        engine
            .refresh(&market)
            .expect(
                "refresh failed"
            );

        let store =
            engine.store();

        let aster = store
            .get(VenueId::Aster)
            .expect(
                "missing Aster book"
            );

        let binance = store
            .get(VenueId::Binance)
            .expect(
                "missing Binance book"
            );

        let edgex = store
            .get(VenueId::EdgeX)
            .expect(
                "missing EdgeX book"
            );

        let okx = store
            .get(VenueId::Okx)
            .expect(
                "missing OKX book"
            );

        assert_eq!(
            aster.venue,
            VenueId::Aster
        );

        assert_eq!(
            binance.venue,
            VenueId::Binance
        );

        assert_eq!(
            edgex.venue,
            VenueId::EdgeX
        );

        assert_eq!(
            okx.venue,
            VenueId::Okx
        );

        /*
         * Verify BID liquidity remains local
         * to each venue.
         */

        assert_eq!(
            aster.bids[0].quantity,
            30
        );

        assert_eq!(
            binance.bids[0].quantity,
            20
        );

        assert_eq!(
            edgex.bids[0].quantity,
            50
        );

        assert_eq!(
            okx.bids[0].quantity,
            75
        );

        /*
         * Verify ASK liquidity remains local
         * to each venue.
         */

        assert_eq!(
            aster.asks[0].quantity,
            20
        );

        assert_eq!(
            binance.asks[0].quantity,
            40
        );

        assert_eq!(
            edgex.asks[0].quantity,
            35
        );

        assert_eq!(
            okx.asks[0].quantity,
            65
        );
    }
}