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

use serde::Deserialize;


// ============================================================
// CONSTANTS
// ============================================================

const FIXED_SCALE: u32 = 8;


// ============================================================
// ASTER
// ============================================================

#[derive(Debug, Clone)]
pub struct Aster {
    pub enabled: bool,
}


impl Aster {

    pub fn new() -> Self {
        Self {
            enabled: true,
        }
    }


    pub fn disabled() -> Self {
        Self {
            enabled: false,
        }
    }


    // ========================================================
    // SYMBOL
    // ========================================================

    fn symbol(
        market: &MarketId,
    ) -> String {

        format!(
            "{}{}",
            market.base.to_uppercase(),
            market.quote.to_uppercase()
        )
    }


    // ========================================================
    // ORDERBOOK
    // ========================================================

    fn fetch_orderbook(
        &self,
        market: &MarketId,
    ) -> Result<VenueOrderBook, VenueError> {

        let symbol =
            Self::symbol(market);


        let url =
            format!(
                "https://fapi.asterdex.com/fapi/v1/depth?symbol={}&limit=100",
                symbol
            );


        let response =
            reqwest::blocking::get(
                &url
            )
            .map_err(|error| {

                VenueError::Internal(
                    format!(
                        "Aster orderbook request failed: {}",
                        error
                    )
                )
            })?;


        if !response.status().is_success() {

            return Err(
                VenueError::Internal(
                    format!(
                        "Aster orderbook HTTP error: {}",
                        response.status()
                    )
                )
            );
        }


        let payload:
            AsterDepthResponse =
            response
                .json()
                .map_err(|error| {

                    VenueError::Internal(
                        format!(
                            "Aster orderbook JSON parse failed: {}",
                            error
                        )
                    )
                })?;


        let bids =
            Self::parse_levels(
                payload.bids
            )?;


        let asks =
            Self::parse_levels(
                payload.asks
            )?;


        if bids.is_empty()
            && asks.is_empty()
        {
            return Err(
                VenueError::Internal(
                    format!(
                        "Aster returned empty orderbook for {}",
                        symbol
                    )
                )
            );
        }


        Ok(
            VenueOrderBook {

                venue:
                    VenueId::Aster,

                market:
                    market.clone(),

                bids,

                asks,

                timestamp_ms:
                    current_timestamp_ms(),
            }
        )
    }


    // ========================================================
    // LAST PRICE
    // ========================================================
    //
    // Aster endpoint:
    //
    // GET /fapi/v1/ticker/price
    //
    // Response:
    //
    // {
    //     "symbol": "BTCUSDT",
    //     "price": "65052.00000000",
    //     "time": 1234567890
    // }
    //
    // Đây là latest ticker price.
    //
    // KHÔNG dùng:
    //
    //     best bid
    //     best ask
    //     mid price
    //
    // vì đó không phải last traded price.
    // ========================================================

    pub fn last_price(
        &self,
        market: &MarketId,
    ) -> Result<u64, VenueError> {

        if !self.enabled {

            return Err(
                VenueError::Internal(
                    "Aster adapter is disabled"
                        .to_string()
                )
            );
        }


        let symbol =
            Self::symbol(market);


        let url =
            format!(
                "https://fapi.asterdex.com/fapi/v1/ticker/price?symbol={}",
                symbol
            );


        let response =
            reqwest::blocking::get(
                &url
            )
            .map_err(|error| {

                VenueError::Internal(
                    format!(
                        "Aster ticker request failed: {}",
                        error
                    )
                )
            })?;


        if !response.status().is_success() {

            return Err(
                VenueError::Internal(
                    format!(
                        "Aster ticker HTTP error: {}",
                        response.status()
                    )
                )
            );
        }


        let payload:
            AsterPriceTickerResponse =
            response
                .json()
                .map_err(|error| {

                    VenueError::Internal(
                        format!(
                            "Aster ticker JSON parse failed: {}",
                            error
                        )
                    )
                })?;


        if payload.symbol
            .to_uppercase()
            != symbol
        {
            return Err(
                VenueError::Internal(
                    format!(
                        "Aster ticker symbol mismatch: expected {}, got {}",
                        symbol,
                        payload.symbol
                    )
                )
            );
        }


        Self::parse_decimal_to_fixed(
            payload.price.trim(),
            "last price",
        )
    }


    // ========================================================
    // ORDERBOOK LEVEL PARSER
    // ========================================================

    fn parse_levels(
        levels: Vec<[String; 2]>,
    ) -> Result<
        Vec<OrderBookLevel>,
        VenueError,
    > {

        let mut result =
            Vec::with_capacity(
                levels.len()
            );


        for level in levels {

            let raw_price =
                level[0].trim();

            let raw_quantity =
                level[1].trim();


            let price =
                Self::parse_decimal_to_fixed(
                    raw_price,
                    "price",
                )?;


            let quantity =
                Self::parse_decimal_to_fixed(
                    raw_quantity,
                    "quantity",
                )?;


            if quantity == 0 {
                continue;
            }


            result.push(
                OrderBookLevel {
                    price,
                    quantity,
                }
            );
        }


        Ok(result)
    }


    // ========================================================
    // DECIMAL -> FIXED POINT
    // ========================================================
    //
    // Scale = 8
    //
    // 65052
    //     -> 6505200000000
    //
    // 65052.45
    //     -> 6505245000000
    //
    // 0.307
    //     -> 30700000
    //
    // Không dùng f64.
    // ========================================================

    fn parse_decimal_to_fixed(
        value: &str,
        field: &str,
    ) -> Result<u64, VenueError> {

        if value.is_empty() {

            return Err(
                VenueError::Internal(
                    format!(
                        "Empty Aster {}",
                        field
                    )
                )
            );
        }


        if value.starts_with('-') {

            return Err(
                VenueError::Internal(
                    format!(
                        "Negative Aster {} is invalid: '{}'",
                        field,
                        value
                    )
                )
            );
        }


        let mut parts =
            value.split('.');


        let integer_part =
            parts.next()
                .unwrap_or("");


        let fractional_part =
            parts.next()
                .unwrap_or("");


        // Chỉ cho phép một dấu '.'
        if parts.next().is_some() {

            return Err(
                VenueError::Internal(
                    format!(
                        "Invalid Aster {} '{}'",
                        field,
                        value
                    )
                )
            );
        }


        if integer_part.is_empty() {

            return Err(
                VenueError::Internal(
                    format!(
                        "Invalid Aster {} '{}'",
                        field,
                        value
                    )
                )
            );
        }


        if !integer_part
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        {
            return Err(
                VenueError::Internal(
                    format!(
                        "Invalid Aster {} '{}'",
                        field,
                        value
                    )
                )
            );
        }


        if !fractional_part
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        {
            return Err(
                VenueError::Internal(
                    format!(
                        "Invalid Aster {} '{}'",
                        field,
                        value
                    )
                )
            );
        }


        if fractional_part.len()
            > FIXED_SCALE as usize
        {
            return Err(
                VenueError::Internal(
                    format!(
                        "Aster {} has more than {} decimal places: '{}'",
                        field,
                        FIXED_SCALE,
                        value
                    )
                )
            );
        }


        let integer =
            integer_part
                .parse::<u64>()
                .map_err(|error| {

                    VenueError::Internal(
                        format!(
                            "Invalid Aster {} '{}': {}",
                            field,
                            value,
                            error
                        )
                    )
                })?;


        let multiplier =
            10u64
                .checked_pow(
                    FIXED_SCALE
                )
                .ok_or_else(|| {

                    VenueError::Internal(
                        "Fixed-point scale overflow"
                            .to_string()
                    )
                })?;


        let integer_scaled =
            integer
                .checked_mul(
                    multiplier
                )
                .ok_or_else(|| {

                    VenueError::Internal(
                        format!(
                            "Aster {} overflow: '{}'",
                            field,
                            value
                        )
                    )
                })?;


        let mut fractional =
            fractional_part
                .parse::<u64>()
                .unwrap_or(0);


        let missing =
            FIXED_SCALE as usize
                - fractional_part.len();


        if missing > 0 {

            let padding =
                10u64
                    .checked_pow(
                        missing as u32
                    )
                    .ok_or_else(|| {

                        VenueError::Internal(
                            "Fractional scale overflow"
                                .to_string()
                        )
                    })?;


            fractional =
                fractional
                    .checked_mul(
                        padding
                    )
                    .ok_or_else(|| {

                        VenueError::Internal(
                            format!(
                                "Aster {} fractional overflow: '{}'",
                                field,
                                value
                            )
                        )
                    })?;
        }


        integer_scaled
            .checked_add(
                fractional
            )
            .ok_or_else(|| {

                VenueError::Internal(
                    format!(
                        "Aster {} fixed-point overflow: '{}'",
                        field,
                        value
                    )
                )
            })
    }
}


// ============================================================
// DEFAULT
// ============================================================

impl Default for Aster {

    fn default() -> Self {
        Self::new()
    }
}


// ============================================================
// VENUE
// ============================================================

impl Venue for Aster {

    fn id(&self) -> VenueId {
        VenueId::Aster
    }


    fn get_orderbook(
        &self,
        market: &MarketId,
    ) -> Result<
        VenueOrderBook,
        VenueError,
    > {

        if !self.enabled {

            return Err(
                VenueError::Internal(
                    "Aster adapter is disabled"
                        .to_string()
                )
            );
        }


        self.fetch_orderbook(
            market
        )
    }


    fn place_order(
        &self,
        _request: &VenueOrderRequest,
    ) -> Result<
        VenueOrder,
        VenueError,
    > {

        Err(
            VenueError::Unsupported(
                "Aster order execution is not connected yet"
                    .to_string()
            )
        )
    }


    fn cancel_order(
        &self,
        _external_order_id: &str,
    ) -> Result<(), VenueError> {

        Err(
            VenueError::Unsupported(
                "Aster order cancellation is not connected yet"
                    .to_string()
            )
        )
    }


    fn get_order(
        &self,
        _external_order_id: &str,
    ) -> Result<
        VenueOrder,
        VenueError,
    > {

        Err(
            VenueError::Unsupported(
                "Aster order query is not connected yet"
                    .to_string()
            )
        )
    }
}


// ============================================================
// MARKET DATA VENUE
// ============================================================

impl MarketDataVenue for Aster {

    fn id(&self) -> VenueId {
        VenueId::Aster
    }


    fn snapshot(
        &self,
        market: &MarketId,
    ) -> Result<
        VenueOrderBook,
        VenueError,
    > {

        self.get_orderbook(
            market
        )
    }
}


// ============================================================
// ASTER API TYPES
// ============================================================

#[derive(Debug, Deserialize)]
struct AsterDepthResponse {

    bids:
        Vec<[String; 2]>,

    asks:
        Vec<[String; 2]>,
}


#[derive(Debug, Deserialize)]
struct AsterPriceTickerResponse {

    symbol:
        String,

    price:
        String,

    #[serde(default)]
    time:
        Option<u64>,
}


// ============================================================
// TIMESTAMP
// ============================================================

fn current_timestamp_ms() -> u64 {

    use std::time::{
        SystemTime,
        UNIX_EPOCH,
    };


    SystemTime::now()
        .duration_since(
            UNIX_EPOCH
        )
        .unwrap_or_default()
        .as_millis() as u64
}


// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn parses_integer_price() {

        assert_eq!(
            Aster::parse_decimal_to_fixed(
                "65052",
                "price"
            )
            .unwrap(),
            65052_00000000
        );
    }


    #[test]
    fn parses_decimal_price() {

        assert_eq!(
            Aster::parse_decimal_to_fixed(
                "65052.45",
                "price"
            )
            .unwrap(),
            65052_45000000
        );
    }


    #[test]
    fn parses_quantity() {

        assert_eq!(
            Aster::parse_decimal_to_fixed(
                "0.307",
                "quantity"
            )
            .unwrap(),
            30_700_000
        );
    }


    #[test]
    fn pads_fractional_digits() {

        assert_eq!(
            Aster::parse_decimal_to_fixed(
                "65052.4",
                "price"
            )
            .unwrap(),
            65052_40000000
        );
    }


    #[test]
    fn zero_is_valid() {

        assert_eq!(
            Aster::parse_decimal_to_fixed(
                "0",
                "price"
            )
            .unwrap(),
            0
        );
    }


    #[test]
    fn rejects_negative() {

        assert!(
            Aster::parse_decimal_to_fixed(
                "-1",
                "price"
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_too_many_decimals() {

        assert!(
            Aster::parse_decimal_to_fixed(
                "65052.123456789",
                "price"
            )
            .is_err()
        );
    }


    #[test]
    fn rejects_invalid_decimal() {

        assert!(
            Aster::parse_decimal_to_fixed(
                "65052.xyz",
                "price"
            )
            .is_err()
        );
    }
}