use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketId {
    pub base: String,
    pub quote: String,
}

impl MarketId {
    pub fn new(
        base: impl Into<String>,
        quote: impl Into<String>,
    ) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
        }
    }

    pub fn symbol(&self) -> String {
        format!(
            "{}/{}",
            self.base,
            self.quote
        )
    }
}

impl fmt::Display for MarketId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}/{}",
            self.base,
            self.quote
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketStatus {
    Active,
    Suspended,
    Offline,
}

#[derive(Debug, Clone)]
pub struct Market {
    pub id: MarketId,
    pub status: MarketStatus,

    /// Minimum price increment in fixed-point units.
    pub tick_size: u64,

    /// Minimum quantity increment in fixed-point units.
    pub lot_size: u64,

    /// Number of decimal places used to represent prices.
    ///
    /// Example:
    /// price_scale = 2
    ///
    /// 65190.25 -> 6519025
    pub price_scale: u32,

    /// Number of decimal places used to represent quantities.
    ///
    /// Example:
    /// quantity_scale = 6
    ///
    /// 0.142 -> 142000
    pub quantity_scale: u32,
}

impl Market {
    pub fn new(
        base: impl Into<String>,
        quote: impl Into<String>,
    ) -> Self {
        Self {
            id: MarketId::new(
                base,
                quote,
            ),

            status: MarketStatus::Active,

            /*
             * Keep the old integer behaviour.
             *
             * Existing tests and markets that use
             * integer prices/quantities continue
             * working exactly as before.
             */
            tick_size: 1,
            lot_size: 1,

            /*
             * Scale 0 means:
             *
             * 65190 -> 65190
             * 142    -> 142
             *
             * This preserves the old representation.
             *
             * Real venues can later configure these
             * explicitly.
             */
            price_scale: 0,
            quantity_scale: 0,
        }
    }

    /// Create a market with explicit decimal precision.
    ///
    /// Example:
    ///
    /// BTC/USDT:
    ///     price_decimals = 2
    ///     quantity_decimals = 6
    pub fn with_precision(
        base: impl Into<String>,
        quote: impl Into<String>,
        price_decimals: u32,
        quantity_decimals: u32,
    ) -> Self {
        Self {
            id: MarketId::new(
                base,
                quote,
            ),

            status: MarketStatus::Active,

            tick_size: 1,
            lot_size: 1,

            price_scale: price_decimals,
            quantity_scale: quantity_decimals,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == MarketStatus::Active
    }

    /// Integer multiplier for price representation.
    ///
    /// Example:
    /// price_scale = 2
    /// -> 100
    pub fn price_multiplier(&self) -> u64 {
        10u64.saturating_pow(
            self.price_scale
        )
    }

    /// Integer multiplier for quantity representation.
    ///
    /// Example:
    /// quantity_scale = 6
    /// -> 1_000_000
    pub fn quantity_multiplier(&self) -> u64 {
        10u64.saturating_pow(
            self.quantity_scale
        )
    }
}