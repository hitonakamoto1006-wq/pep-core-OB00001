use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{OnceLock, RwLock};

use axum::{
    extract::{Path, Query},
    http::Method,
    response::Json,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};

use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::PEPDEX::market::MarketId;
use crate::PEPDEX::market_data::MarketDataEngine;
use crate::PEPDEX::venue::aster::Aster;


// ============================================================
// CONSTANTS
// ============================================================

const FIXED_SCALE: u32 = 8;
const ORDERBOOK_DEPTH: usize = 50;


// ============================================================
// CONNECTED WALLETS
// ============================================================

static CONNECTED_WALLETS:
    OnceLock<RwLock<HashSet<String>>> =
    OnceLock::new();


fn connected_wallets()
    -> &'static RwLock<HashSet<String>>
{
    CONNECTED_WALLETS.get_or_init(|| {
        RwLock::new(HashSet::new())
    })
}


// ============================================================
// WALLET CONNECT REQUEST
// ============================================================

#[derive(Debug, Deserialize)]
struct WalletConnectRequest {

    address: String,
}


// ============================================================
// API RESPONSE
// ============================================================

#[derive(Debug, Serialize)]
pub struct OrderBookResponse {

    pub market: String,

    pub venue_count: usize,

    pub timestamp_ms: u64,

    // --------------------------------------------------------
    // MARKET SUMMARY
    // --------------------------------------------------------

    pub last_price: Option<String>,

    pub best_bid: Option<String>,

    pub best_ask: Option<String>,

    pub spread: Option<String>,

    pub mid_price: Option<String>,

    // --------------------------------------------------------
    // ORDERBOOK
    // --------------------------------------------------------

    pub bids: Vec<OrderBookLevelResponse>,

    pub asks: Vec<OrderBookLevelResponse>,
}


// ============================================================
// ORDERBOOK LEVEL RESPONSE
// ============================================================

#[derive(Debug, Serialize)]
pub struct OrderBookLevelResponse {

    pub price: String,

    pub quantity: String,
}


// ============================================================
// KLINE QUERY
// ============================================================

#[derive(Debug, serde::Deserialize)]
struct KlineQuery {

    #[serde(default = "default_interval")]
    interval: String,

    #[serde(default = "default_limit")]
    limit: usize,
}


fn default_interval() -> String {

    "1m".to_string()
}


fn default_limit() -> usize {

    500
}


// ============================================================
// API SERVER
// ============================================================

pub async fn start() {

    let app =
        Router::new()

            // ------------------------------------------------
            // UI
            // ------------------------------------------------

            .route(
                "/",
                get(index)
            )

            .nest_service(
                "/ui",
                ServeDir::new("ui")
            )

            // ------------------------------------------------
            // API
            // ------------------------------------------------

            .route(
                "/api/market/{base}/{quote}/orderbook",
                get(orderbook)
            )

            .route(
                "/api/market/{base}/{quote}/klines",
                get(klines)
            )

            .route(
                "/api/health",
                get(health)
            )

            // ------------------------------------------------
            // WALLET
            // ------------------------------------------------

            .route(
                "/api/wallet/connect",
                post(connect_wallet)
            )

            .route(
                "/api/wallet/connected",
                get(connected_wallet)
            )

            // ------------------------------------------------
            // CORS
            // ------------------------------------------------

            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                    ])
            );


    let address =
        SocketAddr::from((
            [127, 0, 0, 1],
            3000,
        ));


    println!();
    println!("================================================");
    println!("                 PEPDEX API");
    println!("================================================");

    println!(
        " UI:"
    );

    println!(
        " http://{}/ui/index.html",
        address
    );

    println!();

    println!(
        " Orderbook:"
    );

    println!(
        " http://{}/api/market/BTC/USDT/orderbook",
        address
    );

    println!();

    println!(
        " Klines:"
    );

    println!(
        " http://{}/api/market/BTC/USDT/klines",
        address
    );

    println!();

    println!(
        " Wallet:"
    );

    println!(
        " POST http://{}/api/wallet/connect",
        address
    );

    println!();

    println!(
        " Health:"
    );

    println!(
        " http://{}/api/health",
        address
    );

    println!("================================================");
    println!();


    let listener =
        tokio::net::TcpListener::bind(
            address
        )
        .await
        .expect(
            "failed to bind PEPDEX API"
        );


    axum::serve(
        listener,
        app,
    )
    .await
    .expect(
        "PEPDEX API server failed"
    );
}


// ============================================================
// ROOT
// ============================================================

async fn index() -> axum::response::Redirect {

    axum::response::Redirect::temporary(
        "/ui/index.html"
    )
}


// ============================================================
// HEALTH
// ============================================================

async fn health()
    -> Json<serde_json::Value>
{
    Json(
        serde_json::json!({
            "status": "ok",
            "service": "pepDEX"
        })
    )
}


// ============================================================
// WALLET CONNECT
// ============================================================
//
// Wallet gửi:
//
// {
//     "address": "pep..."
// }
//
// PEPDEX lưu wallet address trong registry
// của process hiện tại.
//
// Sau này registry này sẽ được nối với:
//     Wallet
//     State
//     Asset
//     NAV
//     PEPDEX account
//
// ============================================================

async fn connect_wallet(
    Json(request): Json<WalletConnectRequest>,
) -> Json<serde_json::Value> {

    let address =
        request.address.trim().to_string();


    // --------------------------------------------------------
    // VALIDATE ADDRESS
    // --------------------------------------------------------

    if address.is_empty() {

        return Json(
            serde_json::json!({
                "ok": false,
                "error":
                    "wallet address cannot be empty"
            })
        );
    }


    // --------------------------------------------------------
    // REGISTER WALLET
    // --------------------------------------------------------

    connected_wallets()
        .write()
        .expect(
            "wallet registry poisoned"
        )
        .insert(
            address.clone()
        );


    // --------------------------------------------------------
    // RESPONSE
    // --------------------------------------------------------

    Json(
        serde_json::json!({

            "ok": true,

            "connected": true,

            "address":
                address,

            "service":
                "pepDEX",

        })
    )
}


// ============================================================
// GET CONNECTED WALLET
// ============================================================

async fn connected_wallet()
    -> Json<serde_json::Value>
{
    let wallets =
        connected_wallets()
            .read()
            .expect(
                "wallet registry poisoned"
            );

    match wallets.iter().next() {

        Some(address) => {

            Json(
                serde_json::json!({
                    "ok": true,
                    "connected": true,
                    "address": address
                })
            )
        }

        None => {

            Json(
                serde_json::json!({
                    "ok": true,
                    "connected": false,
                    "address": null
                })
            )
        }
    }
}


// ============================================================
// KLINES
// ============================================================
//
// Endpoint cho TradingView / Chart.js.
//
// GET:
//
// /api/market/BTC/USDT/klines
//
// Query:
//
// interval=1m
// limit=500
//
// ============================================================

async fn klines(
    Path((
        base,
        quote,
    )): Path<(String, String)>,

    Query(query): Query<KlineQuery>,

) -> Json<serde_json::Value> {

    let interval =
        query.interval;

    let limit =
        query.limit.clamp(
            1,
            1500,
        );


    let result =
        tokio::task::spawn_blocking(
            move || {

                fetch_klines(
                    &base,
                    &quote,
                    &interval,
                    limit,
                )
            }
        )
        .await;


    match result {

        // ----------------------------------------------------
        // SUCCESS
        // ----------------------------------------------------

        Ok(
            Ok(data)
        ) => {

            Json(
                serde_json::json!({
                    "ok": true,
                    "data": data
                })
            )
        }


        // ----------------------------------------------------
        // MARKET DATA ERROR
        // ----------------------------------------------------

        Ok(
            Err(error)
        ) => {

            Json(
                serde_json::json!({
                    "ok": false,
                    "error": error
                })
            )
        }


        // ----------------------------------------------------
        // TASK ERROR
        // ----------------------------------------------------

        Err(error) => {

            Json(
                serde_json::json!({
                    "ok": false,
                    "error": format!(
                        "kline task failed: {}",
                        error
                    )
                })
            )
        }
    }
}


// ============================================================
// KLINE FETCH
// ============================================================
//
// Lấy historical OHLCV trực tiếp từ Aster.
//
// Không đụng vào orderbook.
//
// Response được chuyển thành:
//
// {
//     "time":   unix seconds,
//     "open":   "...",
//     "high":   "...",
//     "low":    "...",
//     "close":  "...",
//     "volume": "..."
// }
//
// ============================================================

fn fetch_klines(
    base: &str,
    quote: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<serde_json::Value>, String> {

    let symbol =
        format!(
            "{}{}",
            base.to_uppercase(),
            quote.to_uppercase()
        );


    let url =
        format!(
            "https://fapi.asterdex.com/fapi/v1/klines?symbol={}&interval={}&limit={}",
            symbol,
            interval,
            limit
        );


    let response =
        reqwest::blocking::get(
            &url
        )
        .map_err(|error| {

            format!(
                "Aster kline request failed: {}",
                error
            )
        })?;


    if !response.status().is_success() {

        return Err(
            format!(
                "Aster kline HTTP error: {}",
                response.status()
            )
        );
    }


    let payload:
        Vec<Vec<serde_json::Value>> =
        response
            .json()
            .map_err(|error| {

                format!(
                    "Aster kline JSON parse failed: {}",
                    error
                )
            })?;


    let mut result =
        Vec::with_capacity(
            payload.len()
        );


    for candle in payload {

        if candle.len() < 6 {

            continue;
        }


        // ----------------------------------------------------
        // OPEN TIME
        // ----------------------------------------------------

        let time_ms =
            candle[0]
                .as_u64()
                .ok_or_else(|| {

                    "invalid kline timestamp"
                        .to_string()
                })?;


        // ----------------------------------------------------
        // OPEN
        // ----------------------------------------------------

        let open =
            candle[1]
                .as_str()
                .ok_or_else(|| {

                    "invalid kline open"
                        .to_string()
                })?
                .to_string();


        // ----------------------------------------------------
        // HIGH
        // ----------------------------------------------------

        let high =
            candle[2]
                .as_str()
                .ok_or_else(|| {

                    "invalid kline high"
                        .to_string()
                })?
                .to_string();


        // ----------------------------------------------------
        // LOW
        // ----------------------------------------------------

        let low =
            candle[3]
                .as_str()
                .ok_or_else(|| {

                    "invalid kline low"
                        .to_string()
                })?
                .to_string();


        // ----------------------------------------------------
        // CLOSE
        // ----------------------------------------------------

        let close =
            candle[4]
                .as_str()
                .ok_or_else(|| {

                    "invalid kline close"
                        .to_string()
                })?
                .to_string();


        // ----------------------------------------------------
        // VOLUME
        // ----------------------------------------------------

        let volume =
            candle[5]
                .as_str()
                .ok_or_else(|| {

                    "invalid kline volume"
                        .to_string()
                })?
                .to_string();


        // ----------------------------------------------------
        // CHART RESPONSE
        // ----------------------------------------------------

        result.push(
            serde_json::json!({

                "time":
                    time_ms / 1000,

                "open":
                    open,

                "high":
                    high,

                "low":
                    low,

                "close":
                    close,

                "volume":
                    volume,

            })
        );
    }


    Ok(result)
}


// ============================================================
// ORDERBOOK
// ============================================================

async fn orderbook(
    Path((
        base,
        quote,
    )): Path<(String, String)>,
) -> Json<serde_json::Value> {

    /*
     * HTTP handler chạy async.
     *
     * MarketDataEngine hiện tại dùng blocking
     * venue adapters.
     *
     * Vì vậy market-data được đưa sang
     * blocking thread.
     */

    let result =
        tokio::task::spawn_blocking(
            move || {

                fetch_orderbook(
                    &base,
                    &quote,
                )
            }
        )
        .await;


    match result {

        // ----------------------------------------------------
        // SUCCESS
        // ----------------------------------------------------

        Ok(
            Ok(response)
        ) => {

            Json(
                serde_json::json!({
                    "ok": true,
                    "data": response
                })
            )
        }


        // ----------------------------------------------------
        // MARKET DATA ERROR
        // ----------------------------------------------------

        Ok(
            Err(error)
        ) => {

            Json(
                serde_json::json!({
                    "ok": false,
                    "error": error
                })
            )
        }


        // ----------------------------------------------------
        // TASK ERROR
        // ----------------------------------------------------

        Err(error) => {

            Json(
                serde_json::json!({
                    "ok": false,
                    "error": format!(
                        "market data task failed: {}",
                        error
                    )
                })
            )
        }
    }
}


// ============================================================
// FETCH MARKET DATA
// ============================================================

fn fetch_orderbook(
    base: &str,
    quote: &str,
) -> Result<
    OrderBookResponse,
    String,
> {

    // ========================================================
    // MARKET
    // ========================================================

    let market =
        MarketId::new(
            base,
            quote,
        );


    // ========================================================
    // MARKET DATA ENGINE
    // ========================================================

    let mut engine =
        MarketDataEngine::new();


    /*
     * Aster hiện là market-data venue thật.
     *
     * Sau này có thể mở rộng:
     *
     * engine.add_venue(
     *     Binance::new()
     * );
     *
     * engine.add_venue(
     *     EdgeX::new()
     * );
     *
     * engine.add_venue(
     *     Okx::new()
     * );
     *
     * MarketDataEngine sẽ aggregate
     * liquidity từ nhiều venue.
     */

    let aster =
        Aster::new();


    engine.add_venue(
        aster
    );


    // ========================================================
    // REFRESH ORDERBOOK
    // ========================================================

    let successful =
        engine
            .refresh(
                &market
            )
            .map_err(|error| {

                format!(
                    "market data refresh failed: {:?}",
                    error
                )
            })?;


    // ========================================================
    // AGGREGATED BOOK
    // ========================================================

    let Some(book) =
        engine.aggregated()
    else {

        return Err(
            "no aggregated orderbook available"
                .to_string()
        );
    };


    // ========================================================
    // LAST PRICE
    // ========================================================

    let last_price = {

        let aster =
            Aster::new();


        match aster.last_price(
            &market
        ) {

            Ok(price) => {

                Some(
                    format_fixed(
                        price,
                        FIXED_SCALE,
                    )
                )
            }

            Err(error) => {

                eprintln!(
                    "PEPDEX: Aster last-price error: {:?}",
                    error
                );

                None
            }
        }
    };


    // ========================================================
    // BEST BID
    // ========================================================

    let best_bid =
        book.best_bid()
            .map(|level| {

                format_fixed(
                    level.price,
                    FIXED_SCALE,
                )
            });


    // ========================================================
    // BEST ASK
    // ========================================================

    let best_ask =
        book.best_ask()
            .map(|level| {

                format_fixed(
                    level.price,
                    FIXED_SCALE,
                )
            });


    // ========================================================
    // SPREAD
    // ========================================================

    let spread = {

        match (
            book.best_ask(),
            book.best_bid(),
        ) {

            (
                Some(ask),
                Some(bid),
            ) => {

                Some(
                    format_fixed(
                        ask.price
                            .saturating_sub(
                                bid.price
                            ),
                        FIXED_SCALE,
                    )
                )
            }

            _ => None,
        }
    };


    // ========================================================
    // MID PRICE
    // ========================================================

    let mid_price = {

        match (
            book.best_ask(),
            book.best_bid(),
        ) {

            (
                Some(ask),
                Some(bid),
            ) => {

                let spread =
                    ask.price
                        .saturating_sub(
                            bid.price
                        );


                let mid =
                    bid.price
                        .saturating_add(
                            spread / 2
                        );


                Some(
                    format_fixed(
                        mid,
                        FIXED_SCALE,
                    )
                )
            }

            _ => None,
        }
    };


    // ========================================================
    // BIDS
    // ========================================================

    let bids =
        book.bids
            .iter()
            .take(
                ORDERBOOK_DEPTH
            )
            .map(|level| {

                OrderBookLevelResponse {

                    price:
                        format_fixed(
                            level.price,
                            FIXED_SCALE,
                        ),

                    quantity:
                        format_fixed(
                            level.quantity,
                            FIXED_SCALE,
                        ),
                }
            })
            .collect::<Vec<_>>();


    // ========================================================
    // ASKS
    // ========================================================

    let asks =
        book.asks
            .iter()
            .take(
                ORDERBOOK_DEPTH
            )
            .map(|level| {

                OrderBookLevelResponse {

                    price:
                        format_fixed(
                            level.price,
                            FIXED_SCALE,
                        ),

                    quantity:
                        format_fixed(
                            level.quantity,
                            FIXED_SCALE,
                        ),
                }
            })
            .collect::<Vec<_>>();


    // ========================================================
    // RESPONSE
    // ========================================================

    Ok(
        OrderBookResponse {

            market:
                format!(
                    "{}/{}",
                    base.to_uppercase(),
                    quote.to_uppercase()
                ),

            venue_count:
                successful,

            timestamp_ms:
                book.timestamp_ms,

            last_price,

            best_bid,

            best_ask,

            spread,

            mid_price,

            bids,

            asks,
        }
    )
}


// ============================================================
// FIXED POINT FORMATTER
// ============================================================

fn format_fixed(
    value: u64,
    scale: u32,
) -> String {

    if scale == 0 {

        return value.to_string();
    }


    let divisor =
        10u64
            .checked_pow(
                scale
            )
            .unwrap_or(1);


    let integer =
        value / divisor;


    let fractional =
        value % divisor;


    format!(
        "{}.{:0width$}",
        integer,
        fractional,
        width = scale as usize
    )
}


// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn formats_integer_fixed_point() {

        assert_eq!(
            format_fixed(
                65190,
                0
            ),
            "65190"
        );
    }


    #[test]
    fn formats_decimal_fixed_point() {

        assert_eq!(
            format_fixed(
                6_519_025_000_000,
                8
            ),
            "65190.25000000"
        );
    }


    #[test]
    fn formats_small_decimal() {

        assert_eq!(
            format_fixed(
                14_200_000,
                8
            ),
            "0.14200000"
        );
    }


    #[test]
    fn formats_zero() {

        assert_eq!(
            format_fixed(
                0,
                8
            ),
            "0.00000000"
        );
    }


    #[test]
    fn formats_exact_integer_with_scale() {

        assert_eq!(
            format_fixed(
                65_190_000_000,
                8
            ),
            "651.90000000"
        );
    }


    #[test]
    fn formats_large_price() {

        assert_eq!(
            format_fixed(
                6_505_245_000_000,
                8
            ),
            "65052.45000000"
        );
    }
}