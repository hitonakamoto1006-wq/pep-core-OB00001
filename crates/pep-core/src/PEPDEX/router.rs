use crate::PEPDEX::market::MarketId;
use crate::PEPDEX::order::{
    ChildOrder,
    ExecutionPlan,
    ParentOrder,
};
use crate::PEPDEX::orderbook::{
    AggregatedLevel,
    AggregatedOrderBook,
};
use crate::PEPDEX::venue::VenueOrderSide;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingPolicy {
    BestPrice,
    BestPriceThenLiquidity,
}

#[derive(Debug, Clone)]
pub struct Router {
    pub policy: RoutingPolicy,
}

impl Router {
    pub fn new() -> Self {
        Self {
            policy:
                RoutingPolicy::BestPriceThenLiquidity,
        }
    }

    pub fn with_policy(
        policy: RoutingPolicy,
    ) -> Self {
        Self { policy }
    }

    pub fn route(
        &self,
        order: &ParentOrder,
        book: &AggregatedOrderBook,
    ) -> Result<ExecutionPlan, RouterError> {
        if order.market != book.market {
            return Err(
                RouterError::MarketMismatch {
                    order_market:
                        order.market.clone(),
                    book_market:
                        book.market.clone(),
                },
            );
        }

        if order.quantity == 0 {
            return Err(
                RouterError::InvalidQuantity
            );
        }

        let levels = match order.side {
            VenueOrderSide::Buy => {
                &book.asks
            }

            VenueOrderSide::Sell => {
                &book.bids
            }
        };

        if levels.is_empty() {
            return Err(
                RouterError::NoLiquidity
            );
        }

        let mut remaining =
            order.quantity;

        let mut plan =
            ExecutionPlan::new(
                order.id.clone()
            );

        for level in levels {
            if remaining == 0 {
                break;
            }

            /*
             * Limit order:
             *
             * BUY:
             *   execution price must be <= limit price
             *
             * SELL:
             *   execution price must be >= limit price
             */
            if let Some(limit_price) =
                order.limit_price
            {
                let allowed =
                    match order.side {
                        VenueOrderSide::Buy => {
                            level.price
                                <= limit_price
                        }

                        VenueOrderSide::Sell => {
                            level.price
                                >= limit_price
                        }
                    };

                if !allowed {
                    break;
                }
            }

            let level_quantity =
                level
                    .quantity
                    .min(remaining);

            if level_quantity == 0 {
                continue;
            }

            /*
             * A single aggregated price level
             * may contain liquidity from several
             * venues.
             *
             * Example:
             *
             * 0.6020 × 100
             *   ├── Aster   40
             *   ├── Binance 35
             *   └── OKX     25
             *
             * We turn it into multiple child
             * orders.
             */
            self.split_level(
                order,
                level,
                level_quantity,
                &mut remaining,
                &mut plan,
            );

            if remaining == 0 {
                break;
            }
        }

        /*
         * No child order means that there was
         * genuinely no usable liquidity.
         *
         * This is different from partial routing.
         */
        if plan.children.is_empty() {
            return Err(
                RouterError::InsufficientLiquidity {
                    requested:
                        order.quantity,

                    available:
                        order
                            .quantity
                            .saturating_sub(
                                remaining
                            ),
                },
            );
        }

        /*
         * IMPORTANT:
         *
         * remaining > 0 is NOT an error here.
         *
         * A limit order may be partially routed
         * when the remaining liquidity is outside
         * the allowed price range.
         *
         * Example:
         *
         * Parent = 100
         *
         * 60 @ 602  <- allowed
         * 35 @ 603  <- allowed
         * 55 @ 604  <- rejected by limit 603
         *
         * ExecutionPlan = 95
         * remaining    = 5
         *
         * The execution / aggregation layer will
         * determine the final partial execution
         * state.
         */

        Ok(plan)
    }

    fn split_level(
        &self,
        parent: &ParentOrder,
        level: &AggregatedLevel,
        level_quantity: u64,
        remaining: &mut u64,
        plan: &mut ExecutionPlan,
    ) {
        let mut level_remaining =
            level_quantity;

        for source in &level.sources {
            if level_remaining == 0 {
                break;
            }

            let quantity =
                source
                    .quantity
                    .min(level_remaining);

            if quantity == 0 {
                continue;
            }

            let child_index =
                plan.children.len() + 1;

            let child_id = format!(
                "{}-{}",
                parent.id.as_str(),
                child_index
            );

            let child = ChildOrder::new(
                child_id,
                parent,
                source.venue,
                quantity,
                Some(level.price),
            );

            plan.add_child(child);

            level_remaining -= quantity;
            *remaining -= quantity;
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum RouterError {
    InvalidQuantity,

    NoLiquidity,

    InsufficientLiquidity {
        requested: u64,
        available: u64,
    },

    MarketMismatch {
        order_market: MarketId,
        book_market: MarketId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::order::ParentOrder;
    use crate::PEPDEX::orderbook::{
        AggregatedLevel,
        AggregatedOrderBook,
        LiquiditySource,
    };
    use crate::PEPDEX::venue::{
        VenueId,
        VenueOrderSide,
        VenueOrderType,
    };

    fn test_book()
        -> AggregatedOrderBook
    {
        let market =
            MarketId::new(
                "PEP",
                "USDT"
            );

        AggregatedOrderBook {
            market,

            bids: vec![
                AggregatedLevel {
                    price: 599,
                    quantity: 50,
                    sources: vec![
                        LiquiditySource {
                            venue:
                                VenueId::Aster,
                            quantity: 30,
                        },

                        LiquiditySource {
                            venue:
                                VenueId::Binance,
                            quantity: 20,
                        },
                    ],
                },
            ],

            asks: vec![
                AggregatedLevel {
                    price: 602,
                    quantity: 60,
                    sources: vec![
                        LiquiditySource {
                            venue:
                                VenueId::Aster,
                            quantity: 20,
                        },

                        LiquiditySource {
                            venue:
                                VenueId::Binance,
                            quantity: 40,
                        },
                    ],
                },

                AggregatedLevel {
                    price: 603,
                    quantity: 35,
                    sources: vec![
                        LiquiditySource {
                            venue:
                                VenueId::EdgeX,
                            quantity: 35,
                        },
                    ],
                },

                AggregatedLevel {
                    price: 604,
                    quantity: 55,
                    sources: vec![
                        LiquiditySource {
                            venue:
                                VenueId::Okx,
                            quantity: 55,
                        },
                    ],
                },
            ],

            timestamp_ms: 0,
        }
    }

    #[test]
    fn buy_order_is_split_across_venues()
    {
        let market =
            MarketId::new(
                "PEP",
                "USDT"
            );

        let order =
            ParentOrder::new(
                "ORDER-1",
                "alice",
                market,
                VenueOrderSide::Buy,
                VenueOrderType::Market,
                100,
                None,
            );

        let book =
            test_book();

        let router =
            Router::new();

        let plan = router
            .route(
                &order,
                &book
            )
            .expect(
                "routing failed"
            );

        /*
         * 60 @ 602
         *   Aster   20
         *   Binance 40
         *
         * 35 @ 603
         *   EdgeX 35
         *
         * Remaining 5 @ 604
         *   OKX 5
         */

        assert_eq!(
            plan.total_quantity(),
            100
        );

        assert_eq!(
            plan.child_count(),
            4
        );

        assert_eq!(
            plan.children[0].venue,
            VenueId::Aster
        );

        assert_eq!(
            plan.children[0].quantity,
            20
        );

        assert_eq!(
            plan.children[1].venue,
            VenueId::Binance
        );

        assert_eq!(
            plan.children[1].quantity,
            40
        );

        assert_eq!(
            plan.children[2].venue,
            VenueId::EdgeX
        );

        assert_eq!(
            plan.children[2].quantity,
            35
        );

        assert_eq!(
            plan.children[3].venue,
            VenueId::Okx
        );

        assert_eq!(
            plan.children[3].quantity,
            5
        );
    }

    #[test]
    fn sell_order_uses_bids()
    {
        let market =
            MarketId::new(
                "PEP",
                "USDT"
            );

        let order =
            ParentOrder::new(
                "ORDER-2",
                "alice",
                market,
                VenueOrderSide::Sell,
                VenueOrderType::Market,
                50,
                None,
            );

        let book =
            test_book();

        let router =
            Router::new();

        let plan = router
            .route(
                &order,
                &book
            )
            .expect(
                "routing failed"
            );

        assert_eq!(
            plan.total_quantity(),
            50
        );

        assert_eq!(
            plan.child_count(),
            2
        );

        assert_eq!(
            plan.children[0].venue,
            VenueId::Aster
        );

        assert_eq!(
            plan.children[0].quantity,
            30
        );

        assert_eq!(
            plan.children[1].venue,
            VenueId::Binance
        );

        assert_eq!(
            plan.children[1].quantity,
            20
        );
    }

    #[test]
    fn limit_order_respects_price()
    {
        let market =
            MarketId::new(
                "PEP",
                "USDT"
            );

        let order =
            ParentOrder::new(
                "ORDER-3",
                "alice",
                market,
                VenueOrderSide::Buy,
                VenueOrderType::Limit,
                100,
                Some(603),
            );

        let book =
            test_book();

        let router =
            Router::new();

        let plan = router
            .route(
                &order,
                &book
            )
            .expect(
                "routing failed"
            );

        /*
         * 60 @ 602
         * 35 @ 603
         *
         * Total = 95
         *
         * 604 is above the 603 limit,
         * therefore it must NOT be routed.
         */
        assert_eq!(
            plan.total_quantity(),
            95
        );

        assert_eq!(
            plan.child_count(),
            3
        );
    }

    #[test]
    fn rejects_when_no_liquidity()
    {
        let market =
            MarketId::new(
                "PEP",
                "USDT"
            );

        let order =
            ParentOrder::new(
                "ORDER-4",
                "alice",
                market,
                VenueOrderSide::Buy,
                VenueOrderType::Market,
                1,
                None,
            );

        let mut book =
            test_book();

        book.asks.clear();

        let router =
            Router::new();

        let result =
            router.route(
                &order,
                &book
            );

        assert!(matches!(
            result,
            Err(
                RouterError::NoLiquidity
            )
        ));
    }
}