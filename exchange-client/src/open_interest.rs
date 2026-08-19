use futures_util::{stream, StreamExt};
use serde::Deserialize;
use shared::OpenInterest;
use tokio::time::{sleep, Duration};
use tracing::warn;
use crate::{config::ExchangeClientConfig, error::{ExchangeClientError, Result}, RedisPublisher};
#[derive(Debug, Deserialize)]struct BinanceOpenInterest{symbol:String,#[serde(rename="openInterest")]open_interest:String,time:i64}
pub async fn fetch_open_interest(config:&ExchangeClientConfig,symbol:&str)->Result<OpenInterest>{let url=format!("{}/fapi/v1/openInterest?symbol={}",config.futures_rest_base,symbol);let value:BinanceOpenInterest=reqwest::Client::new().get(url).send().await?.error_for_status()?.json().await?;let parsed=value.open_interest.parse::<f64>().map_err(|_|ExchangeClientError::InvalidMarketData("invalid open interest".into()))?;if !parsed.is_finite()||parsed<0.0{return Err(ExchangeClientError::InvalidMarketData("invalid open interest".into()));}Ok(OpenInterest{symbol:value.symbol,timestamp:value.time,value:parsed})}

/// Binance's open-interest REST endpoint takes one symbol per request and
/// has no combined/batched form, so a large symbol universe is fetched
/// concurrently (bounded, to stay well under the futures API's rate limit)
/// rather than one at a time, which would otherwise make each poll cycle
/// take minutes once the universe grows past a handful of symbols.
const CONCURRENT_REQUESTS: usize = 20;

pub async fn run_open_interest_poll(config: &ExchangeClientConfig, publisher: &mut RedisPublisher) {
    loop {
        let results: Vec<(String, Result<OpenInterest>)> = stream::iter(config.symbols.clone())
            .map(|symbol| async move {
                let result = fetch_open_interest(config, &symbol).await;
                (symbol, result)
            })
            .buffer_unordered(CONCURRENT_REQUESTS)
            .collect()
            .await;
        for (symbol, result) in results {
            match result {
                Ok(value) => {
                    if let Err(error) = publisher.publish_open_interest(&value).await {
                        warn!(%error, %symbol, "failed to publish open interest");
                    }
                }
                Err(error) => warn!(%error, %symbol, "failed to fetch open interest"),
            }
        }
        sleep(Duration::from_secs(30)).await;
    }
}
