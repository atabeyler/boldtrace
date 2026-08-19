//! Connects to Binance's spot combined WebSocket stream for kline and
//! order book depth updates, and republishes them to Redis.

use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::backoff::ReconnectBackoff;
use crate::binance_messages::{CombinedStreamEnvelope, DepthPayload, KlineEvent};
use crate::config::ExchangeClientConfig;
use crate::error::Result;
use crate::redis_publisher::RedisPublisher;

fn current_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn spot_stream_url(config: &ExchangeClientConfig) -> String {
    let streams = config
        .symbols
        .iter()
        .map(|s| {
            let symbol = s.to_lowercase();
            format!("{symbol}@kline_1m/{symbol}@kline_5m/{symbol}@depth20@1000ms")
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("{}?streams={streams}", config.spot_ws_base)
}

/// Runs the spot stream connection forever, reconnecting with exponential
/// backoff whenever the connection drops.
pub async fn run_spot_stream(
    config: &ExchangeClientConfig,
    publisher: &mut RedisPublisher,
) -> Result<()> {
    let mut backoff = ReconnectBackoff::default();
    loop {
        match run_spot_stream_once(config, publisher).await {
            Ok(()) => backoff.reset(),
            Err(err) => {
                tracing::warn!(error = %err, "spot stream connection lost, reconnecting");
            }
        }
        tokio::time::sleep(backoff.next_delay()).await;
    }
}

async fn run_spot_stream_once(
    config: &ExchangeClientConfig,
    publisher: &mut RedisPublisher,
) -> Result<()> {
    let url = spot_stream_url(config);
    tracing::info!(%url, "connecting to Binance spot stream");
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        let message = message?;
        match message {
            Message::Text(text) => {
                handle_spot_message(&text, publisher).await;
            }
            Message::Ping(payload) => {
                write.send(Message::Pong(payload)).await?;
            }
            Message::Close(_) => {
                return Err(crate::error::ExchangeClientError::ConnectionClosed);
            }
            _ => {}
        }
    }

    Err(crate::error::ExchangeClientError::ConnectionClosed)
}

async fn handle_spot_message(text: &str, publisher: &mut RedisPublisher) {
    let envelope: CombinedStreamEnvelope<serde_json::Value> = match serde_json::from_str(text) {
        Ok(envelope) => envelope,
        Err(err) => {
            tracing::warn!(error = %err, "failed to parse spot stream envelope");
            return;
        }
    };

    if envelope.stream.contains("@kline") {
        match serde_json::from_value::<KlineEvent>(envelope.data) {
            Ok(event) => match event.into_candle() {
                Ok(candle) => {
                    if let Err(err) = publisher.publish_candle(&candle).await {
                        tracing::warn!(error = %err, "failed to publish candle");
                    }
                }
                Err(err) => tracing::warn!(error = %err, "failed to parse candle numeric fields"),
            },
            Err(err) => tracing::warn!(error = %err, "failed to parse kline event"),
        }
    } else if envelope.stream.contains("@depth") {
        let symbol = envelope
            .stream
            .split('@')
            .next()
            .unwrap_or_default()
            .to_uppercase();
        match serde_json::from_value::<DepthPayload>(envelope.data) {
            Ok(payload) => match payload.into_snapshot(symbol, current_millis()) {
                Ok(snapshot) => {
                    if let Err(err) = publisher.publish_order_book(&snapshot).await {
                        tracing::warn!(error = %err, "failed to publish order book snapshot");
                    }
                }
                Err(err) => tracing::warn!(error = %err, "failed to parse depth numeric fields"),
            },
            Err(err) => tracing::warn!(error = %err, "failed to parse depth payload"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_stream_url() {
        let config = ExchangeClientConfig::new(vec!["BTCUSDT".into()], "redis://127.0.0.1:6379");
        let url = spot_stream_url(&config);
        assert_eq!(
            url,
            "wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/btcusdt@kline_5m/btcusdt@depth20@1000ms"
        );
    }

    #[test]
    fn builds_combined_stream_url_for_multiple_symbols() {
        let config = ExchangeClientConfig::new(
            vec!["BTCUSDT".into(), "ETHUSDT".into()],
            "redis://127.0.0.1:6379",
        );
        let url = spot_stream_url(&config);
        assert_eq!(
            url,
            "wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/btcusdt@kline_5m/btcusdt@depth20@1000ms/ethusdt@kline_1m/ethusdt@kline_5m/ethusdt@depth20@1000ms"
        );
    }
}
