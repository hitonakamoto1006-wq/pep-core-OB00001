use std::collections::BTreeMap;

use crate::PEPDEX::market::MarketId;
use crate::PEPDEX::venue::{
    OrderBookLevel,
    VenueId,
    VenueOrderBook,
};

#[derive(Debug, Clone)]
pub struct LiquiditySource {
    pub venue: VenueId,
    pub quantity: u64,
}

#[derive(Debug, Clone)]
pub struct AggregatedLevel {
    pub price: u64,
    pub quantity: u64,
    pub sources: Vec<LiquiditySource>,
}

#[derive(Debug, Clone)]
pub struct AggregatedOrderBook {
    pub market: MarketId,
    pub bids: Vec<AggregatedLevel>,
    pub asks: Vec<AggregatedLevel>,
    pub timestamp_ms: u64,
}

impl AggregatedOrderBook {
    pub fn best_bid(&self) -> Option<&AggregatedLevel> {
        self.bids.first()
    }

    pub fn best_ask(&self) -> Option<&AggregatedLevel> {
        self.asks.first()
    }
}

fn aggregate_side(
    books: &[VenueOrderBook],
    buy_side: bool,
) -> Vec<AggregatedLevel> {
    // price -> venue -> quantity
    let mut levels: BTreeMap<
        u64,
        BTreeMap<VenueId, u64>,
    > = BTreeMap::new();

    for book in books {
        let source: &[OrderBookLevel] = if buy_side {
            &book.bids
        } else {
            &book.asks
        };

        for level in source {
            let venue_levels =
                levels.entry(level.price).or_default();

            *venue_levels
                .entry(book.venue)
                .or_insert(0) += level.quantity;
        }
    }

    let iter = if buy_side {
        Box::new(levels.into_iter().rev())
            as Box<
                dyn Iterator<
                    Item = (u64, BTreeMap<VenueId, u64>)
                >,
            >
    } else {
        Box::new(levels.into_iter())
            as Box<
                dyn Iterator<
                    Item = (u64, BTreeMap<VenueId, u64>)
                >,
            >
    };

    iter.map(|(price, venue_levels)| {
        let sources: Vec<LiquiditySource> =
            venue_levels
                .into_iter()
                .map(|(venue, quantity)| {
                    LiquiditySource {
                        venue,
                        quantity,
                    }
                })
                .collect();

        let quantity =
            sources.iter().map(|s| s.quantity).sum();

        AggregatedLevel {
            price,
            quantity,
            sources,
        }
    })
    .collect()
}

pub fn aggregate_orderbooks(
    books: &[VenueOrderBook],
) -> Option<AggregatedOrderBook> {
    let first = books.first()?;

    let bids = aggregate_side(books, true);
    let asks = aggregate_side(books, false);

    let timestamp_ms = books
        .iter()
        .map(|book| book.timestamp_ms)
        .max()
        .unwrap_or(0);

    Some(AggregatedOrderBook {
        market: first.market.clone(),
        bids,
        asks,
        timestamp_ms,
    })
}