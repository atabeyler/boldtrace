use serde::Deserialize;
use shared::OpenInterest;
use tokio::time::{sleep, Duration};
use tracing::warn;

use crate::{config::ExchangeClientConfig, error::Result, RedisPublisher};

#[derive(Debug, Deserialize)]
struct BinanceOpenInterest {
    symbol: String,
    #[serde(rename = "openInterest")]
    open_interest: String,
    time: i64,
}

pub async fn fetch_open_interest(config: &ExchangeClientConfig) -> Result<OpenInterest> {
    let url = format!("{}/fapi/v1/openInterest?symbol={}", config.futures_rest_base, config.symbol);
    let value: BinanceOpenInterest = reqwest::Client::new().get(url).send().await?.error_for_status()?.json().await?;
    Ok(OpenInterest { symbol: value.symbol, timestamp: value.time, value: value.open_interest.parse().unwrap_or(0.0) })
}

pub async fn run_open_interest_poll(config: &ExchangeClientConfig, publisher: &mut RedisPublisher) {
    loop {
        match fetch_open_interest(config).await {
            Ok(value) => {
                if let Err(error) = publisher.publish_open_interest(&value).await { warn!(%error, "failed to publish open interest"); }
            }
            Err(error) => warn!(%error, "failed to fetch open interest"),
        }
        sleep(Duration::from_secs(30)).await;
    }
}
