//! WebSocket and REST connectivity to exchange market data (Binance).

mod backoff;
mod binance_messages;
mod config;
mod error;
mod funding_stream;
mod open_interest;
mod redis_publisher;
mod spot_stream;

pub use config::ExchangeClientConfig;
pub use error::{ExchangeClientError, Result};
pub use funding_stream::fetch_funding_rate_history;
pub use open_interest::fetch_open_interest;
pub use redis_publisher::RedisPublisher;

pub async fn run(config: ExchangeClientConfig) -> Result<()> {
    let mut spot_publisher = RedisPublisher::connect(&config.redis_url).await?;
    let mut funding_publisher = RedisPublisher::connect(&config.redis_url).await?;
    let mut oi_publisher = RedisPublisher::connect(&config.redis_url).await?;
    let spot_config = config.clone();
    let funding_config = config.clone();
    let oi_config = config.clone();
    let spot_handle = tokio::spawn(async move { spot_stream::run_spot_stream(&spot_config, &mut spot_publisher).await });
    let funding_handle = tokio::spawn(async move { funding_stream::run_funding_stream(&funding_config, &mut funding_publisher).await });
    let oi_handle = tokio::spawn(async move { open_interest::run_open_interest_poll(&oi_config, &mut oi_publisher).await });
    let _ = tokio::join!(spot_handle, funding_handle, oi_handle);
    Ok(())
}
