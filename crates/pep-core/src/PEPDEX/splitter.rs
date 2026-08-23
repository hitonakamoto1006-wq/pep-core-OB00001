use crate::PEPDEX::market::Market;
use crate::PEPDEX::order::{
    ChildOrder,
    ExecutionPlan,
    OrderId,
    ParentOrder,
};
use crate::PEPDEX::venue::{
    VenueId,
    VenueOrderType,
};

#[derive(Debug, Clone, Copy)]
pub struct SplitRules {
    /// Minimum quantity that can be sent to a venue.
    pub lot_size: u64,

    /// Minimum price increment accepted by a venue.
    pub tick_size: u64,

    /// If true, a remainder smaller than lot_size is rejected.
    pub reject_remainder: bool,
}

impl Default for SplitRules {
    fn default() -> Self {
        Self {
            lot_size: 1,
            tick_size: 1,
            reject_remainder: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Splitter {
    pub default_rules: SplitRules,
}

impl Splitter {
    pub fn new() -> Self {
        Self {
            default_rules: SplitRules::default(),
        }
    }

    pub fn with_rules(rules: SplitRules) -> Self {
        Self {
            default_rules: rules,
        }
    }

    /// Normalize an execution plan before it reaches the executor.
    ///
    /// Router:
    ///     decides WHERE and roughly HOW MUCH.
    ///
    /// Splitter:
    ///     makes every child order valid for execution.
    pub fn split(
        &self,
        plan: &ExecutionPlan,
        market: &Market,
    ) -> Result<ExecutionPlan, SplitterError> {
        if !market.is_active() {
            return Err(SplitterError::MarketInactive);
        }

        let rules = SplitRules {
            lot_size: if market.lot_size == 0 {
                self.default_rules.lot_size
            } else {
                market.lot_size
            },

            tick_size: if market.tick_size == 0 {
                self.default_rules.tick_size
            } else {
                market.tick_size
            },

            reject_remainder: self
                .default_rules
                .reject_remainder,
        };

        let mut output =
            ExecutionPlan::new(plan.parent_order_id.clone());

        for child in &plan.children {
            let normalized =
                self.normalize_child(child, rules)?;

            if let Some(child) = normalized {
                output.add_child(child);
            }
        }

        if output.children.is_empty() {
            return Err(SplitterError::NothingExecutable);
        }

        Ok(output)
    }

    fn normalize_child(
        &self,
        child: &ChildOrder,
        rules: SplitRules,
    ) -> Result<Option<ChildOrder>, SplitterError> {
        if child.quantity == 0 {
            return Ok(None);
        }

        let quantity = normalize_quantity(
            child.quantity,
            rules.lot_size,
        );

        if quantity == 0 {
            if rules.reject_remainder {
                return Err(
                    SplitterError::QuantityBelowLotSize {
                        venue: child.venue,
                        quantity: child.quantity,
                        lot_size: rules.lot_size,
                    },
                );
            }

            return Ok(None);
        }

        if child.order_type == VenueOrderType::Limit {
            if let Some(price) = child.price {
                let normalized_price =
                    normalize_price(
                        price,
                        rules.tick_size,
                    );

                let mut new_child = child.clone();

                new_child.quantity = quantity;
                new_child.price =
                    Some(normalized_price);

                return Ok(Some(new_child));
            }
        }

        let mut new_child = child.clone();
        new_child.quantity = quantity;

        Ok(Some(new_child))
    }
}

impl Default for Splitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Round quantity DOWN to a valid lot size.
///
/// Example:
///
/// quantity = 107
/// lot_size = 10
///
/// result = 100
fn normalize_quantity(
    quantity: u64,
    lot_size: u64,
) -> u64 {
    if lot_size <= 1 {
        return quantity;
    }

    (quantity / lot_size) * lot_size
}

/// Round price DOWN to the nearest valid tick.
///
/// Example:
///
/// price = 6037
/// tick = 10
///
/// result = 6030
fn normalize_price(
    price: u64,
    tick_size: u64,
) -> u64 {
    if tick_size <= 1 {
        return price;
    }

    (price / tick_size) * tick_size
}

#[derive(Debug)]
pub enum SplitterError {
    MarketInactive,

    NothingExecutable,

    QuantityBelowLotSize {
        venue: VenueId,
        quantity: u64,
        lot_size: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEPDEX::market::{
        Market,
    };
    use crate::PEPDEX::order::{
        ChildOrder,
        ExecutionPlan,
        ParentOrder,
    };
    use crate::PEPDEX::venue::{
        VenueId,
        VenueOrderSide,
        VenueOrderType,
    };

    fn parent_order() -> ParentOrder {
        ParentOrder::new(
            "PARENT-1",
            "alice",
            crate::PEPDEX::market::MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            VenueOrderType::Limit,
            100,
            Some(603),
        )
    }

    #[test]
    fn quantity_is_rounded_down_to_lot_size() {
        let parent = parent_order();

        let child = ChildOrder::new(
            "CHILD-1",
            &parent,
            VenueId::Aster,
            107,
            Some(603),
        );

        let mut plan =
            ExecutionPlan::new(parent.id.clone());

        plan.add_child(child);

        let mut market = Market::new(
            "PEP",
            "USDT",
        );

        market.lot_size = 10;
        market.tick_size = 1;

        let splitter = Splitter::new();

        let result = splitter
            .split(&plan, &market)
            .expect("split failed");

        assert_eq!(
            result.children.len(),
            1
        );

        assert_eq!(
            result.children[0].quantity,
            100
        );
    }

    #[test]
    fn price_is_rounded_down_to_tick_size() {
        let parent = parent_order();

        let child = ChildOrder::new(
            "CHILD-1",
            &parent,
            VenueId::Aster,
            100,
            Some(6037),
        );

        let mut plan =
            ExecutionPlan::new(parent.id.clone());

        plan.add_child(child);

        let mut market = Market::new(
            "PEP",
            "USDT",
        );

        market.lot_size = 1;
        market.tick_size = 10;

        let splitter = Splitter::new();

        let result = splitter
            .split(&plan, &market)
            .expect("split failed");

        assert_eq!(
            result.children[0].price,
            Some(6030)
        );
    }

    #[test]
    fn quantity_below_lot_size_is_rejected() {
        let parent = parent_order();

        let child = ChildOrder::new(
            "CHILD-1",
            &parent,
            VenueId::Aster,
            5,
            Some(603),
        );

        let mut plan =
            ExecutionPlan::new(parent.id.clone());

        plan.add_child(child);

        let mut market = Market::new(
            "PEP",
            "USDT",
        );

        market.lot_size = 10;

        let splitter = Splitter::new();

        let result =
            splitter.split(&plan, &market);

        assert!(matches!(
            result,
            Err(
                SplitterError::QuantityBelowLotSize {
                    ..
                }
            )
        ));
    }

    #[test]
    fn market_order_does_not_require_price() {
        let mut parent = ParentOrder::new(
            "PARENT-2",
            "alice",
            crate::PEPDEX::market::MarketId::new(
                "PEP",
                "USDT",
            ),
            VenueOrderSide::Buy,
            VenueOrderType::Market,
            100,
            None,
        );

        parent.limit_price = None;

        let child = ChildOrder::new(
            "CHILD-1",
            &parent,
            VenueId::Binance,
            100,
            None,
        );

        let mut plan =
            ExecutionPlan::new(parent.id.clone());

        plan.add_child(child);

        let mut market = Market::new(
            "PEP",
            "USDT",
        );

        market.lot_size = 10;

        let splitter = Splitter::new();

        let result = splitter
            .split(&plan, &market)
            .expect("split failed");

        assert_eq!(
            result.children[0].quantity,
            100
        );

        assert_eq!(
            result.children[0].price,
            None
        );
    }

    #[test]
    fn inactive_market_is_rejected() {
        let parent = parent_order();

        let child = ChildOrder::new(
            "CHILD-1",
            &parent,
            VenueId::Aster,
            100,
            Some(603),
        );

        let mut plan =
            ExecutionPlan::new(parent.id.clone());

        plan.add_child(child);

        let mut market = Market::new(
            "PEP",
            "USDT",
        );

        market.status =
            crate::PEPDEX::market::MarketStatus::Offline;

        let splitter = Splitter::new();

        let result =
            splitter.split(&plan, &market);

        assert!(matches!(
            result,
            Err(SplitterError::MarketInactive)
        ));
    }
}