//! Bybit V5 public linear-market adapter.
//!
//! Uses one public linear WebSocket for closed 1m/5m klines, level-50
//! order-book updates and tickers. Tickers supply funding rate and open
//! interest. All converted values are published through the same shared
//! Redis channels used by the Binance adapter.

use crate::backoff::ReconnectBackoff;
use crate::config::ExchangeClientConfig;
use crate::error::{ExchangeClientError, Result};
use crate::redis_publisher::RedisPublisher;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{Candle, FundingRate, OpenInterest, OrderBookLevel, OrderBookSnapshot};
use std::collections::HashMap;
use tokio::time::{interval, Duration};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Deserialize)]
struct InstrumentsResponse {
    #[serde(rename = "retCode")]
    ret_code: i64,
    result: InstrumentsResult,
}

#[derive(Debug, Deserialize)]
struct InstrumentsResult {
    list: Vec<Instrument>,
    #[serde(rename = "nextPageCursor")]
    next_page_cursor: String,
}

#[derive(Debug, Deserialize)]
struct Instrument {
    symbol: String,
    #[serde(rename = "contractType")]
    contract_type: String,
    status: String,
    #[serde(rename = "quoteCoin")]
    quote_coin: String,
}

#[derive(Default)]
struct BookState {
    bids: HashMap<String, String>,
    asks: HashMap<String, String>,
}

pub async fn fetch_bybit_perpetual_symbols(config: &ExchangeClientConfig) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let mut cursor = String::new();
    let mut symbols = Vec::new();
    loop {
        let mut request = client
            .get(format!("{}/v5/market/instruments-info", config.bybit_rest_base))
            .query(&[("category", "linear"), ("status", "Trading"), ("limit", "1000")]);
        if !cursor.is_empty() {
            request = request.query(&[("cursor", cursor.as_str())]);
        }
        let response: InstrumentsResponse = request.send().await?.error_for_status()?.json().await?;
        if response.ret_code != 0 {
            return Err(ExchangeClientError::InvalidPayload(format!(
                "Bybit instruments response retCode={}", response.ret_code
            )));
        }
        symbols.extend(response.result.list.into_iter().filter_map(|instrument| {
            (instrument.status == "Trading"
                && instrument.contract_type == "LinearPerpetual"
                && instrument.quote_coin == "USDT")
                .then_some(instrument.symbol)
        }));
        cursor = response.result.next_page_cursor;
        if cursor.is_empty() {
            break;
        }
    }
    symbols.sort();
    symbols.dedup();
    Ok(symbols)
}

pub async fn run_bybit_stream(
    config: &ExchangeClientConfig,
    publisher: &mut RedisPublisher,
) -> Result<()> {
    let mut backoff = ReconnectBackoff::default();
    loop {
        match run_bybit_stream_once(config, publisher).await {
            Ok(()) => backoff.reset(),
            Err(err) => tracing::warn!(error=%err, "Bybit stream disconnected, reconnecting"),
        }
        tokio::time::sleep(backoff.next_delay()).await;
    }
}

async fn run_bybit_stream_once(
    config: &ExchangeClientConfig,
    publisher: &mut RedisPublisher,
) -> Result<()> {
    let (stream, _) = tokio_tungstenite::connect_async(&config.bybit_ws_base).await?;
    let (mut write, mut read) = stream.split();
    let args = config
        .symbols
        .iter()
        .flat_map(|symbol| {
            [
                format!("kline.1.{symbol}"),
                format!("kline.5.{symbol}"),
                format!("orderbook.50.{symbol}"),
                format!("tickers.{symbol}"),
            ]
        })
        .collect::<Vec<_>>();
    write
        .send(Message::Text(json!({"op":"subscribe","args":args}).to_string()))
        .await?;
    tracing::info!(count=config.symbols.len(), "connected to Bybit public linear stream");

    let mut books: HashMap<String, BookState> = HashMap::new();
    let mut heartbeat = interval(Duration::from_secs(20));
    heartbeat.tick().await;

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                write.send(Message::Text(json!({"op":"ping"}).to_string())).await?;
            }
            message = read.next() => {
                let Some(message) = message else { return Err(ExchangeClientError::ConnectionClosed); };
                match message? {
                    Message::Text(text) => handle_bybit_message(&text, publisher, &mut books).await,
                    Message::Ping(payload) => write.send(Message::Pong(payload)).await?,
                    Message::Close(_) => return Err(ExchangeClientError::ConnectionClosed),
                    _ => {}
                }
            }
        }
    }
}

async fn handle_bybit_message(
    text: &str,
    publisher: &mut RedisPublisher,
    books: &mut HashMap<String, BookState>,
) {
    let Ok(value) = serde_json::from_str::<Value>(text) else { return; };
    let Some(topic) = value.get("topic").and_then(Value::as_str) else { return; };
    let timestamp = value.get("ts").and_then(Value::as_i64).unwrap_or_default();
    if topic.starts_with("kline.") {
        handle_kline(topic, timestamp, &value, publisher).await;
    } else if topic.starts_with("orderbook.") {
        handle_order_book(timestamp, &value, publisher, books).await;
    } else if topic.starts_with("tickers.") {
        handle_ticker(timestamp, &value, publisher).await;
    }
}

async fn handle_kline(topic: &str, _timestamp: i64, value: &Value, publisher: &mut RedisPublisher) {
    let Some(row) = value.get("data").and_then(Value::as_array).and_then(|rows| rows.first()) else { return; };
    if !row.get("confirm").and_then(Value::as_bool).unwrap_or(false) { return; }
    let symbol = topic.rsplit('.').next().unwrap_or_default().to_string();
    let interval = row.get("interval").and_then(Value::as_str).unwrap_or_default();
    let interval = match interval { "1" => "1m", "5" => "5m", other => other };
    let parse = |key: &str| row.get(key).and_then(Value::as_str)?.parse::<f64>().ok();
    let candle = Candle {
        symbol,
        interval: interval.to_string(),
        open_time: row.get("start").and_then(Value::as_i64).unwrap_or_default(),
        close_time: row.get("end").and_then(Value::as_i64).unwrap_or_default(),
        open: match parse("open") { Some(v) => v, None => return },
        high: match parse("high") { Some(v) => v, None => return },
        low: match parse("low") { Some(v) => v, None => return },
        close: match parse("close") { Some(v) => v, None => return },
        volume: match parse("volume") { Some(v) => v, None => return },
    };
    if let Err(err) = publisher.publish_candle(&candle).await {
        tracing::warn!(error=%err, "failed to publish Bybit candle");
    }
}

async fn handle_order_book(
    timestamp: i64,
    value: &Value,
    publisher: &mut RedisPublisher,
    books: &mut HashMap<String, BookState>,
) {
    let Some(data) = value.get("data") else { return; };
    let symbol = data.get("s").and_then(Value::as_str).unwrap_or_default().to_string();
    if symbol.is_empty() { return; }
    let state = books.entry(symbol.clone()).or_default();
    if value.get("type").and_then(Value::as_str) == Some("snapshot") {
        state.bids.clear();
        state.asks.clear();
    }
    apply_levels(&mut state.bids, data.get("b"));
    apply_levels(&mut state.asks, data.get("a"));
    let mut bids = materialize_levels(&state.bids, true);
    let mut asks = materialize_levels(&state.asks, false);
    bids.truncate(50);
    asks.truncate(50);
    let snapshot = OrderBookSnapshot { symbol, timestamp, bids, asks };
    if let Err(err) = publisher.publish_order_book(&snapshot).await {
        tracing::warn!(error=%err, "failed to publish Bybit order book");
    }
}

fn apply_levels(book: &mut HashMap<String, String>, rows: Option<&Value>) {
    let Some(rows) = rows.and_then(Value::as_array) else { return; };
    for row in rows {
        let Some(parts) = row.as_array() else { continue; };
        let Some(price) = parts.first().and_then(Value::as_str) else { continue; };
        let Some(quantity) = parts.get(1).and_then(Value::as_str) else { continue; };
        if quantity.parse::<f64>().ok().is_some_and(|value| value == 0.0) {
            book.remove(price);
        } else {
            book.insert(price.to_string(), quantity.to_string());
        }
    }
}

fn materialize_levels(book: &HashMap<String, String>, descending: bool) -> Vec<OrderBookLevel> {
    let mut levels = book
        .iter()
        .filter_map(|(price, quantity)| Some(OrderBookLevel {
            price: price.parse().ok()?,
            quantity: quantity.parse().ok()?,
        }))
        .collect::<Vec<_>>();
    levels.sort_by(|a, b| if descending { b.price.total_cmp(&a.price) } else { a.price.total_cmp(&b.price) });
    levels
}

async fn handle_ticker(timestamp: i64, value: &Value, publisher: &mut RedisPublisher) {
    let Some(data) = value.get("data") else { return; };
    let symbol = data.get("symbol").and_then(Value::as_str).unwrap_or_default().to_string();
    if symbol.is_empty() { return; }
    if let Some(rate) = data.get("fundingRate").and_then(Value::as_str).and_then(|v| v.parse::<f64>().ok()) {
        let funding = FundingRate { symbol: symbol.clone(), timestamp, rate };
        if let Err(err) = publisher.publish_funding_rate(&funding).await {
            tracing::warn!(error=%err, "failed to publish Bybit funding rate");
        }
    }
    if let Some(value) = data.get("openInterest").and_then(Value::as_str).and_then(|v| v.parse::<f64>().ok()) {
        let oi = OpenInterest { symbol, timestamp, value };
        if let Err(err) = publisher.publish_open_interest(&oi).await {
            tracing::warn!(error=%err, "failed to publish Bybit open interest");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materializes_book_in_market_order() {
        let mut book = HashMap::new();
        book.insert("100".to_string(), "2".to_string());
        book.insert("101".to_string(), "3".to_string());
        assert_eq!(materialize_levels(&book, true)[0].price, 101.0);
        assert_eq!(materialize_levels(&book, false)[0].price, 100.0);
    }

    #[test]
    fn zero_quantity_removes_level() {
        let mut book = HashMap::new();
        book.insert("100".to_string(), "2".to_string());
        let row = json!([["100", "0"]]);
        apply_levels(&mut book, Some(&row));
        assert!(book.is_empty());
    }
}
