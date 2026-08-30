//! Real OHLCV candle history for the web dashboard's price chart.
//!
//! Persisted in the `candles` table (see `migrations/0008_candles.sql`) from
//! two sources, both real exchange data, never fabricated:
//! - a one-time REST backfill against Binance's public spot klines endpoint
//!   at startup, so the chart has history immediately instead of waiting
//!   hours for live candles to accumulate;
//! - an ongoing subscription to exchange-client's `candles:{symbol}:{interval}`
//!   Redis pub/sub channel, upserting every closed candle it publishes.
//!
//! The HTTP handler only ever reads from Postgres: an unavailable database
//! or an empty table returns an empty list, matching the rest of the API's
//! "unavailable data is shown as unavailable" contract.

use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;

const SUPPORTED_INTERVALS: [&str; 2] = ["1m", "5m"];
const BACKFILL_LIMIT: u32 = 500;

#[derive(Debug, Deserialize)]
pub struct CandleQuery {
    pub interval: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandleItem {
    pub open_time: i64,
    pub close_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(sqlx::FromRow)]
struct Row {
    open_time_millis: i64,
    close_time_millis: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

pub async fn candles(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(q): Query<CandleQuery>,
) -> Json<Vec<CandleItem>> {
    let symbol = symbol.to_uppercase();
    if symbol.len() > 24 || !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Json(Vec::new());
    }
    let interval = q.interval.unwrap_or_else(|| "1m".into());
    if !SUPPORTED_INTERVALS.contains(&interval.as_str()) {
        return Json(Vec::new());
    }
    let Some(pool) = state.pool.clone() else {
        return Json(Vec::new());
    };
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);

    let rows: Result<Vec<Row>, sqlx::Error> = sqlx::query_as(
        "SELECT open_time_millis, close_time_millis, open, high, low, close, volume \
         FROM candles WHERE symbol = $1 AND interval = $2 \
         ORDER BY open_time_millis DESC LIMIT $3",
    )
    .bind(&symbol)
    .bind(&interval)
    .bind(limit)
    .fetch_all(&pool)
    .await;

    let mut rows = match rows {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!(error=%err, %symbol, %interval, "candles query failed");
            Vec::new()
        }
    };
    rows.reverse(); // oldest-first, the order a chart draws in

    Json(
        rows.into_iter()
            .map(|r| CandleItem {
                open_time: r.open_time_millis,
                close_time: r.close_time_millis,
                open: r.open,
                high: r.high,
                low: r.low,
                close: r.close,
                volume: r.volume,
            })
            .collect(),
    )
}

/// Spawns the background backfill + live-ingestion tasks. Best-effort: any
/// failure is logged and retried, never surfaced as a panic, since candle
/// history is a UI enhancement rather than a critical-path dependency.
pub fn spawn_ingestion(pool: PgPool, redis_url: String, symbols: Vec<String>) {
    let backfill_pool = pool.clone();
    let backfill_symbols = symbols;
    tokio::spawn(async move {
        for symbol in &backfill_symbols {
            for interval in SUPPORTED_INTERVALS {
                if let Err(err) = backfill_symbol(&backfill_pool, symbol, interval).await {
                    tracing::warn!(error = %err, %symbol, interval, "candle backfill failed");
                }
            }
        }
    });

    tokio::spawn(async move {
        loop {
            if let Err(err) = run_subscription_once(&pool, &redis_url).await {
                tracing::warn!(error = %err, "candle redis subscription lost, reconnecting");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn backfill_symbol(pool: &PgPool, symbol: &str, interval: &str) -> Result<(), String> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={symbol}&interval={interval}&limit={BACKFILL_LIMIT}"
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| err.to_string())?;
    let rows: Vec<serde_json::Value> = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .json()
        .await
        .map_err(|err| err.to_string())?;

    for row in rows {
        let arr = row.as_array().ok_or("unexpected kline row shape")?;
        let field_str = |i: usize| -> Result<f64, String> {
            arr.get(i)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| format!("missing/invalid kline field {i}"))
        };
        let open_time = arr.first().and_then(|v| v.as_i64()).ok_or("missing open time")?;
        let open = field_str(1)?;
        let high = field_str(2)?;
        let low = field_str(3)?;
        let close = field_str(4)?;
        let volume = field_str(5)?;
        let close_time = arr.get(6).and_then(|v| v.as_i64()).ok_or("missing close time")?;
        upsert_candle(pool, symbol, interval, open_time, close_time, open, high, low, close, volume)
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn run_subscription_once(pool: &PgPool, redis_url: &str) -> Result<(), String> {
    let client = redis::Client::open(redis_url).map_err(|err| err.to_string())?;
    let mut pubsub = client.get_async_pubsub().await.map_err(|err| err.to_string())?;
    pubsub.psubscribe("candles:*").await.map_err(|err| err.to_string())?;
    let mut stream = pubsub.on_message();
    while let Some(msg) = futures_util::StreamExt::next(&mut stream).await {
        let payload: String = match msg.get_payload() {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        let candle: shared::Candle = match serde_json::from_str(&payload) {
            Ok(candle) => candle,
            Err(_) => continue,
        };
        if let Err(err) = upsert_candle(
            pool,
            &candle.symbol,
            &candle.interval,
            candle.open_time,
            candle.close_time,
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume,
        )
        .await
        {
            tracing::warn!(error = %err, symbol = %candle.symbol, "failed to persist live candle");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn upsert_candle(
    pool: &PgPool,
    symbol: &str,
    interval: &str,
    open_time: i64,
    close_time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO candles (symbol, interval, open_time_millis, close_time_millis, open, high, low, close, volume) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
         ON CONFLICT (symbol, interval, open_time_millis) \
         DO UPDATE SET close_time_millis = EXCLUDED.close_time_millis, open = EXCLUDED.open, \
             high = EXCLUDED.high, low = EXCLUDED.low, close = EXCLUDED.close, volume = EXCLUDED.volume",
    )
    .bind(symbol)
    .bind(interval)
    .bind(open_time)
    .bind(close_time)
    .bind(open)
    .bind(high)
    .bind(low)
    .bind(close)
    .bind(volume)
    .execute(pool)
    .await?;
    Ok(())
}
