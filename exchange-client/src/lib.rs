//! WebSocket and REST connectivity to exchange market data (Binance).
//!
//! Connects to Binance's public spot stream for kline and order book
//! depth data and to the futures stream for funding rate, deserializes
//! incoming messages into `shared` domain types, and republishes them to
//! Redis for downstream consumers (`score-engine`, `bot`).

mod backoff;
mod binance_messages;
mod config;
mod error;
mod funding_stream;
mod redis_publisher;
mod spot_stream;

pub use config::ExchangeClientConfig;
pub use error::{ExchangeClientError, Result};
pub use funding_stream::fetch_funding_rate_history;
pub use redis_publisher::RedisPublisher;

/// Runs the spot and futures stream pipelines concurrently, forever. Each
/// pipeline reconnects independently on failure; this function only
/// returns if the underlying Redis connections cannot be established.
pub async fn run(config: ExchangeClientConfig) -> Result<()> {
    let mut spot_publisher = RedisPublisher::connect(&config.redis_url).await?;
    let mut funding_publisher = RedisPublisher::connect(&config.redis_url).await?;

    let spot_config = config.clone();
    let funding_config = config.clone();

    let spot_handle = tokio::spawn(async move {
        spot_stream::run_spot_stream(&spot_config, &mut spot_publisher).await
    });
    let funding_handle = tokio::spawn(async move {
        funding_stream::run_funding_stream(&funding_config, &mut funding_publisher).await
    });

    let _ = tokio::try_join!(spot_handle, funding_handle);
    Ok(())
}
