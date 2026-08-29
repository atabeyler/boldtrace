//! Connects to Binance spot combined WebSocket streams for closed klines and
//! republishes candles to Redis. Order-book depth is sourced from perpetual
//! futures in `funding_stream.rs` so microstructure and derivatives inputs
//! describe the same leveraged venue.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::backoff::ReconnectBackoff;
use crate::binance_messages::{CombinedStreamEnvelope, KlineEvent};
use crate::config::ExchangeClientConfig;
use crate::error::Result;
use crate::redis_publisher::RedisPublisher;

fn candle_stream_url(config: &ExchangeClientConfig) -> String {
    let streams = config
        .symbols
        .iter()
        .map(|s| {
            let symbol = s.to_lowercase();
            format!("{symbol}@kline_1m/{symbol}@kline_5m")
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("{}?streams={streams}", config.spot_ws_base)
}

pub async fn run_candle_stream(config: &ExchangeClientConfig, publisher: &mut RedisPublisher) -> Result<()> {
    let mut backoff = ReconnectBackoff::default();
    loop {
        match run_candle_stream_once(config, publisher).await {
            Ok(()) => backoff.reset(),
            Err(err) => tracing::warn!(error = %err, "candle stream connection lost, reconnecting"),
        }
        tokio::time::sleep(backoff.next_delay()).await;
    }
}

async fn run_candle_stream_once(config: &ExchangeClientConfig, publisher: &mut RedisPublisher) -> Result<()> {
    let url = candle_stream_url(config);
    tracing::info!(%url, "connecting to Binance spot candle stream");
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();
    while let Some(message) = read.next().await {
        match message? {
            Message::Text(text) => handle_candle_message(&text, publisher).await,
            Message::Ping(payload) => write.send(Message::Pong(payload)).await?,
            Message::Close(_) => return Err(crate::error::ExchangeClientError::ConnectionClosed),
            _ => {}
        }
    }
    Err(crate::error::ExchangeClientError::ConnectionClosed)
}

async fn handle_candle_message(text: &str, publisher: &mut RedisPublisher) {
    let envelope: CombinedStreamEnvelope<serde_json::Value> = match serde_json::from_str(text) {
        Ok(envelope) => envelope,
        Err(err) => {
            tracing::warn!(error = %err, "failed to parse candle stream envelope");
            return;
        }
    };
    if !envelope.stream.contains("@kline") {
        return;
    }
    match serde_json::from_value::<KlineEvent>(envelope.data) {
        Ok(event) if event.is_closed() => match event.into_candle() {
            Ok(candle) => {
                if let Err(err) = publisher.publish_candle(&candle).await {
                    tracing::warn!(error = %err, "failed to publish candle");
                }
            }
            Err(err) => tracing::warn!(error = %err, "failed to parse candle numeric fields"),
        },
        Ok(_) => {
            // Ignore forming-candle updates. Decisions are based on immutable
            // closed observations and therefore cannot repaint after alerting.
        }
        Err(err) => tracing::warn!(error = %err, "failed to parse kline event"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_expected_stream_url() {
        let config = ExchangeClientConfig::new(vec!["BTCUSDT".into()], "redis://127.0.0.1:6379");
        assert_eq!(candle_stream_url(&config), "wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/btcusdt@kline_5m");
    }
    #[test]
    fn builds_combined_stream_url_for_multiple_symbols() {
        let config = ExchangeClientConfig::new(vec!["BTCUSDT".into(), "ETHUSDT".into()], "redis://127.0.0.1:6379");
        assert_eq!(candle_stream_url(&config), "wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/btcusdt@kline_5m/ethusdt@kline_1m/ethusdt@kline_5m");
    }
}
