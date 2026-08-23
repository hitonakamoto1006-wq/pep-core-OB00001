use crate::PEPDEX::market_data::MarketDataEngine;
use crate::PEPDEX::order::ParentOrder;
use crate::PEPDEX::venue::VenueOrderSide;

#[derive(Debug, Clone, Copy)]
pub struct OrderConstraints {
    /// Maximum quote asset the user is willing to spend.
    ///
    /// Mainly used for BUY orders.
    pub max_spend: Option<u64>,

    /// Maximum acceptable slippage in basis points.
    ///
    /// 100 bps = 1%.
    pub max_slippage_bps: Option<u64>,

    /// Maximum quantity the user is willing to sell.
    ///
    /// Mainly useful for SELL-side risk validation.
    pub max_quantity: Option<u64>,
}

impl Default for OrderConstraints {
    fn default() -> Self {
        Self {
            max_spend: None,
            max_slippage_bps: None,
            max_quantity: None,
        }
    }
}

impl OrderConstraints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_spend(
        mut self,
        amount: u64,
    ) -> Self {
        self.max_spend = Some(amount);
        self
    }

    pub fn with_max_slippage_bps(
        mut self,
        bps: u64,
    ) -> Self {
        self.max_slippage_bps = Some(bps);
        self
    }

    pub fn with_max_quantity(
        mut self,
        quantity: u64,
    ) -> Self {
        self.max_quantity = Some(quantity);
        self
    }
}

#[derive(Debug, Clone)]
pub struct RiskContext {
    pub constraints: OrderConstraints,

    /// Reference price obtained from the current
    /// aggregated market data.
    pub reference_price: Option<u64>,
}

impl RiskContext {
    pub fn new(
        constraints: OrderConstraints,
    ) -> Self {
        Self {
            constraints,
            reference_price: None,
        }
    }
}

pub struct RiskEngine;

impl RiskEngine {
    pub fn new() -> Self {
        Self
    }

    /// Validate order constraints before routing.
    pub fn validate(
        &self,
        order: &ParentOrder,
        constraints: &OrderConstraints,
    ) -> Result<(), RiskError> {
        if order.quantity == 0 {
            return Err(
                RiskError::ZeroQuantity
            );
        }

        if let Some(max_quantity) =
            constraints.max_quantity
        {
            if order.quantity > max_quantity {
                return Err(
                    RiskError::QuantityLimitExceeded {
                        requested:
                            order.quantity,
                        maximum:
                            max_quantity,
                    }
                );
            }
        }

        match order.side {
            VenueOrderSide::Buy => {
                self.validate_buy(
                    order,
                    constraints,
                )?;
            }

            VenueOrderSide::Sell => {
                self.validate_sell(
                    order,
                    constraints,
                )?;
            }
        }

        if let Some(
            max_slippage
        ) = constraints.max_slippage_bps {
            if max_slippage > 10_000 {
                return Err(
                    RiskError::InvalidSlippage
                );
            }
        }

        Ok(())
    }

    fn validate_buy(
        &self,
        order: &ParentOrder,
        constraints: &OrderConstraints,
    ) -> Result<(), RiskError> {
        /*
         * LIMIT BUY:
         *
         * quantity × limit_price must fit inside
         * max_spend if max_spend was supplied.
         */
        if let Some(limit_price) =
            order.limit_price
        {
            let maximum_cost =
                order
                    .quantity
                    .checked_mul(
                        limit_price
                    )
                    .ok_or(
                        RiskError
                            ::ArithmeticOverflow
                    )?;

            if let Some(max_spend) =
                constraints.max_spend
            {
                if maximum_cost > max_spend {
                    return Err(
                        RiskError
                            ::MaxSpendExceeded {
                                estimated:
                                    maximum_cost,
                                maximum:
                                    max_spend,
                            }
                    );
                }
            }
        } else {
            /*
             * MARKET BUY MUST have either:
             *
             * max_spend
             *
             * or
             *
             * max_slippage_bps
             *
             * Otherwise the order has no meaningful
             * execution boundary.
             */
            if constraints.max_spend.is_none()
                && constraints
                    .max_slippage_bps
                    .is_none()
            {
                return Err(
                    RiskError
                        ::MarketBuyRequiresConstraint
                );
            }
        }

        Ok(())
    }

    fn validate_sell(
        &self,
        order: &ParentOrder,
        constraints: &OrderConstraints,
    ) -> Result<(), RiskError> {
        if let Some(max_quantity) =
            constraints.max_quantity
        {
            if order.quantity > max_quantity {
                return Err(
                    RiskError::QuantityLimitExceeded {
                        requested:
                            order.quantity,
                        maximum:
                            max_quantity,
                    }
                );
            }
        }

        Ok(())
    }

    /// Validate the actual execution against the user's
    /// maximum spend after routing.
    pub fn validate_execution_cost(
        &self,
        order: &ParentOrder,
        constraints: &OrderConstraints,
        actual_cost: u64,
        reference_price: Option<u64>,
    ) -> Result<(), RiskError> {
        if order.side
            != VenueOrderSide::Buy
        {
            return Ok(());
        }

        /*
         * Hard maximum spend always wins.
         */
        if let Some(max_spend) =
            constraints.max_spend
        {
            if actual_cost > max_spend {
                return Err(
                    RiskError
                        ::ExecutionExceededMaxSpend {
                            actual:
                                actual_cost,
                            maximum:
                                max_spend,
                        }
                );
            }
        }

        /*
         * Slippage check.
         */
        if let (
            Some(max_slippage_bps),
            Some(reference),
        ) = (
            constraints.max_slippage_bps,
            reference_price,
        ) {
            if reference == 0 {
                return Err(
                    RiskError
                        ::InvalidReferencePrice
                );
            }

            let executed_price =
                actual_cost
                    / order.quantity.max(1);

            let maximum_price =
                reference
                    .saturating_mul(
                        10_000u64
                            .saturating_add(
                                max_slippage_bps
                            )
                    )
                    / 10_000;

            if executed_price
                > maximum_price
            {
                return Err(
                    RiskError
                        ::SlippageExceeded {
                            reference_price:
                                reference,

                            execution_price:
                                executed_price,

                            maximum_price,
                        }
                );
            }
        }

        Ok(())
    }

    /// Obtain a reference price from the current
    /// aggregated orderbook.
    pub fn reference_price(
        &self,
        order: &ParentOrder,
        market_data: &MarketDataEngine,
    ) -> Result<u64, RiskError> {
        let book =
            market_data
                .aggregated()
                .ok_or(
                    RiskError
                        ::NoMarketData
                )?;

        match order.side {
            VenueOrderSide::Buy => {
                book.best_ask()
                    .map(|level| level.price)
                    .ok_or(
                        RiskError
                            ::NoMarketData
                    )
            }

            VenueOrderSide::Sell => {
                book.best_bid()
                    .map(|level| level.price)
                    .ok_or(
                        RiskError
                            ::NoMarketData
                    )
            }
        }
    }
}

impl Default for RiskEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum RiskError {
    ZeroQuantity,

    QuantityLimitExceeded {
        requested: u64,
        maximum: u64,
    },

    MaxSpendExceeded {
        estimated: u64,
        maximum: u64,
    },

    ExecutionExceededMaxSpend {
        actual: u64,
        maximum: u64,
    },

    MarketBuyRequiresConstraint,

    InvalidSlippage,

    SlippageExceeded {
        reference_price: u64,
        execution_price: u64,
        maximum_price: u64,
    },

    InvalidReferencePrice,

    NoMarketData,

    ArithmeticOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::MarketId;
    use crate::PEPDEX::order::ParentOrder;
    use crate::PEPDEX::venue::{
        VenueOrderSide,
        VenueOrderType,
    };

    fn limit_buy() -> ParentOrder {
        ParentOrder::new(
            "ORDER-1",
            "alice",
            MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            VenueOrderType::Limit,
            100,
            Some(600),
        )
    }

    fn market_buy() -> ParentOrder {
        ParentOrder::new(
            "ORDER-2",
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
    fn limit_buy_respects_max_spend() {
        let engine =
            RiskEngine::new();

        let order =
            limit_buy();

        let constraints =
            OrderConstraints::new()
                .with_max_spend(60_000);

        engine
            .validate(
                &order,
                &constraints,
            )
            .expect(
                "order should pass"
            );
    }

    #[test]
    fn limit_buy_rejects_excessive_spend() {
        let engine =
            RiskEngine::new();

        let order =
            limit_buy();

        let constraints =
            OrderConstraints::new()
                .with_max_spend(50_000);

        let result =
            engine.validate(
                &order,
                &constraints,
            );

        assert!(matches!(
            result,
            Err(
                RiskError
                    ::MaxSpendExceeded { .. }
            )
        ));
    }

    #[test]
    fn market_buy_requires_constraint() {
        let engine =
            RiskEngine::new();

        let order =
            market_buy();

        let constraints =
            OrderConstraints::new();

        let result =
            engine.validate(
                &order,
                &constraints,
            );

        assert!(matches!(
            result,
            Err(
                RiskError
                    ::MarketBuyRequiresConstraint
            )
        ));
    }

    #[test]
    fn market_buy_accepts_max_spend() {
        let engine =
            RiskEngine::new();

        let order =
            market_buy();

        let constraints =
            OrderConstraints::new()
                .with_max_spend(
                    70_000
                );

        engine
            .validate(
                &order,
                &constraints,
            )
            .expect(
                "market buy should pass"
            );
    }

    #[test]
    fn market_buy_accepts_slippage_limit() {
        let engine =
            RiskEngine::new();

        let order =
            market_buy();

        let constraints =
            OrderConstraints::new()
                .with_max_slippage_bps(
                    100
                );

        engine
            .validate(
                &order,
                &constraints,
            )
            .expect(
                "market buy should pass"
            );
    }

    #[test]
    fn execution_cost_cannot_exceed_max_spend() {
        let engine =
            RiskEngine::new();

        let order =
            market_buy();

        let constraints =
            OrderConstraints::new()
                .with_max_spend(
                    60_000
                );

        let result =
            engine.validate_execution_cost(
                &order,
                &constraints,
                60_001,
                None,
            );

        assert!(matches!(
            result,
            Err(
                RiskError
                    ::ExecutionExceededMaxSpend {
                        ..
                    }
            )
        ));
    }

    #[test]
    fn slippage_is_checked() {
        let engine =
            RiskEngine::new();

        let order =
            market_buy();

        let constraints =
            OrderConstraints::new()
                .with_max_slippage_bps(
                    100
                );

        /*
         * Reference = 600
         * Maximum = 606
         *
         * Actual = 607
         * => reject
         */
        let result =
            engine.validate_execution_cost(
                &order,
                &constraints,
                60_700,
                Some(600),
            );

        assert!(matches!(
            result,
            Err(
                RiskError
                    ::SlippageExceeded {
                        ..
                    }
            )
        ));
    }

    #[test]
    fn invalid_slippage_is_rejected() {
        let engine =
            RiskEngine::new();

        let order =
            limit_buy();

        let constraints =
            OrderConstraints::new()
                .with_max_slippage_bps(
                    10_001
                );

        let result =
            engine.validate(
                &order,
                &constraints,
            );

        assert!(matches!(
            result,
            Err(
                RiskError::InvalidSlippage
            )
        ));
    }
}