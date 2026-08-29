//! Connects to Binance USDⓈ-M futures WebSocket for mark-price/funding and
//! futures order-book depth.  Keeping depth on the perpetual venue avoids
//! mixing spot microstructure with futures funding/open-interest signals.

use std::time::{SystemTime, UNIX_EPOCH};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::backoff::ReconnectBackoff;
use crate::binance_messages::{CombinedStreamEnvelope, DepthPayload, FundingRateHistoryEntry, MarkPriceEvent};
use crate::config::ExchangeClientConfig;
use crate::error::Result;
use crate::redis_publisher::RedisPublisher;

fn current_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

fn funding_stream_url(config: &ExchangeClientConfig) -> String {
    let streams = config
        .symbols
        .iter()
        .map(|s| {
            let symbol = s.to_lowercase();
            format!("{symbol}@markPrice@1s/{symbol}@depth20@1000ms")
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("{}?streams={streams}", config.futures_ws_base)
}

pub async fn run_funding_stream(config: &ExchangeClientConfig, publisher: &mut RedisPublisher) -> Result<()> {
    let mut backoff = ReconnectBackoff::default();
    loop {
        match run_funding_stream_once(config, publisher).await {
            Ok(()) => backoff.reset(),
            Err(err) => tracing::warn!(error = %err, "futures funding/depth stream connection lost, reconnecting"),
        }
        tokio::time::sleep(backoff.next_delay()).await;
    }
}

async fn run_funding_stream_once(config: &ExchangeClientConfig, publisher: &mut RedisPublisher) -> Result<()> {
    let url = funding_stream_url(config);
    tracing::info!(%url, "connecting to Binance futures funding/depth stream");
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();
    while let Some(message) = read.next().await {
        match message? {
            Message::Text(text) => handle_futures_message(&text, publisher).await,
            Message::Ping(payload) => write.send(Message::Pong(payload)).await?,
            Message::Close(_) => return Err(crate::error::ExchangeClientError::ConnectionClosed),
            _ => {}
        }
    }
    Err(crate::error::ExchangeClientError::ConnectionClosed)
}

async fn handle_futures_message(text: &str, publisher: &mut RedisPublisher) {
    let envelope: CombinedStreamEnvelope<serde_json::Value> = match serde_json::from_str(text) {
        Ok(envelope) => envelope,
        Err(err) => { tracing::warn!(error = %err, "failed to parse futures stream envelope"); return; }
    };
    if envelope.stream.contains("@depth") {
        let symbol = envelope.stream.split('@').next().unwrap_or_default().to_uppercase();
        match serde_json::from_value::<DepthPayload>(envelope.data) {
            Ok(payload) => match payload.into_snapshot(symbol, current_millis()) {
                Ok(snapshot) => {
                    if let Err(err) = publisher.publish_order_book(&snapshot).await {
                        tracing::warn!(error = %err, "failed to publish futures order book snapshot");
                    }
                }
                Err(err) => tracing::warn!(error = %err, "failed to parse futures depth numeric fields"),
            },
            Err(err) => tracing::warn!(error = %err, "failed to parse futures depth payload"),
        }
        return;
    }

    match serde_json::from_value::<MarkPriceEvent>(envelope.data) {
        Ok(event) => match event.into_funding_rate() {
            Ok(funding_rate) => {
                if let Err(err) = publisher.publish_funding_rate(&funding_rate).await {
                    tracing::warn!(error = %err, "failed to publish funding rate");
                }
            }
            Err(err) => tracing::warn!(error = %err, "failed to parse funding rate numeric field"),
        },
        Err(err) => tracing::warn!(error = %err, "failed to parse mark price event"),
    }
}

pub async fn fetch_funding_rate_history(config: &ExchangeClientConfig, symbol: &str, limit: u32) -> Result<Vec<shared::FundingRate>> {
    let url = format!("{}/fapi/v1/fundingRate?symbol={}&limit={}", config.futures_rest_base, symbol, limit);
    let entries: Vec<FundingRateHistoryEntry> = reqwest::get(url).await?.json().await?;
    Ok(entries.into_iter().filter_map(|entry| entry.into_funding_rate().ok()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_expected_stream_url() {
        let config = ExchangeClientConfig::new(vec!["BTCUSDT".into()], "redis://127.0.0.1:6379");
        assert_eq!(funding_stream_url(&config), "wss://fstream.binance.com/stream?streams=btcusdt@markPrice@1s/btcusdt@depth20@1000ms");
    }
    #[test]
    fn builds_combined_stream_url_for_multiple_symbols() {
        let config = ExchangeClientConfig::new(vec!["BTCUSDT".into(), "ETHUSDT".into()], "redis://127.0.0.1:6379");
        assert_eq!(funding_stream_url(&config), "wss://fstream.binance.com/stream?streams=btcusdt@markPrice@1s/btcusdt@depth20@1000ms/ethusdt@markPrice@1s/ethusdt@depth20@1000ms");
    }
}
