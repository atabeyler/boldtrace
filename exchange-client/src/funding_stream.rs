//! Connects to Binance's USDT-margined futures WebSocket for real-time
//! funding rate updates, and provides a REST fallback for funding rate
//! history. Funding rate is a perpetual-futures concept and is not
//! available on the spot market.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::backoff::ReconnectBackoff;
use crate::binance_messages::{CombinedStreamEnvelope, FundingRateHistoryEntry, MarkPriceEvent};
use crate::config::ExchangeClientConfig;
use crate::error::Result;
use crate::redis_publisher::RedisPublisher;

fn funding_stream_url(config: &ExchangeClientConfig) -> String {
    format!(
        "{}?streams={}@markPrice@1s",
        config.futures_ws_base,
        config.symbol_lower()
    )
}

/// Runs the futures mark price stream forever, reconnecting with
/// exponential backoff whenever the connection drops.
pub async fn run_funding_stream(
    config: &ExchangeClientConfig,
    publisher: &mut RedisPublisher,
) -> Result<()> {
    let mut backoff = ReconnectBackoff::default();
    loop {
        match run_funding_stream_once(config, publisher).await {
            Ok(()) => backoff.reset(),
            Err(err) => {
                tracing::warn!(error = %err, "funding stream connection lost, reconnecting");
            }
        }
        tokio::time::sleep(backoff.next_delay()).await;
    }
}

async fn run_funding_stream_once(
    config: &ExchangeClientConfig,
    publisher: &mut RedisPublisher,
) -> Result<()> {
    let url = funding_stream_url(config);
    tracing::info!(%url, "connecting to Binance futures mark price stream");
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        let message = message?;
        match message {
            Message::Text(text) => {
                handle_funding_message(&text, publisher).await;
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

async fn handle_funding_message(text: &str, publisher: &mut RedisPublisher) {
    let envelope: CombinedStreamEnvelope<serde_json::Value> = match serde_json::from_str(text) {
        Ok(envelope) => envelope,
        Err(err) => {
            tracing::warn!(error = %err, "failed to parse funding stream envelope");
            return;
        }
    };

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

/// Fetches recent funding rate history over REST, used as a fallback when
/// the WebSocket stream is unavailable or to backfill history on startup.
pub async fn fetch_funding_rate_history(
    config: &ExchangeClientConfig,
    limit: u32,
) -> Result<Vec<shared::FundingRate>> {
    let url = format!(
        "{}/fapi/v1/fundingRate?symbol={}&limit={}",
        config.futures_rest_base, config.symbol, limit
    );
    let entries: Vec<FundingRateHistoryEntry> = reqwest::get(url).await?.json().await?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| entry.into_funding_rate().ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_stream_url() {
        let config = ExchangeClientConfig::new("BTCUSDT", "redis://127.0.0.1:6379");
        let url = funding_stream_url(&config);
        assert_eq!(
            url,
            "wss://fstream.binance.com/stream?streams=btcusdt@markPrice@1s"
        );
    }
}
