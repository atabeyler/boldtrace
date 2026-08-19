use serde::Deserialize;
use shared::OpenInterest;
use tokio::time::{sleep, Duration};
use tracing::warn;
use crate::{config::ExchangeClientConfig, error::{ExchangeClientError, Result}, RedisPublisher};
#[derive(Debug, Deserialize)]struct BinanceOpenInterest{symbol:String,#[serde(rename="openInterest")]open_interest:String,time:i64}
pub async fn fetch_open_interest(config:&ExchangeClientConfig,symbol:&str)->Result<OpenInterest>{let url=format!("{}/fapi/v1/openInterest?symbol={}",config.futures_rest_base,symbol);let value:BinanceOpenInterest=reqwest::Client::new().get(url).send().await?.error_for_status()?.json().await?;let parsed=value.open_interest.parse::<f64>().map_err(|_|ExchangeClientError::InvalidMarketData("invalid open interest".into()))?;if !parsed.is_finite()||parsed<0.0{return Err(ExchangeClientError::InvalidMarketData("invalid open interest".into()));}Ok(OpenInterest{symbol:value.symbol,timestamp:value.time,value:parsed})}
/// Polls open interest for every configured symbol in turn, once per
/// 30-second tick, since Binance's open-interest REST endpoint takes a
/// single symbol per request (there's no combined/batched form).
pub async fn run_open_interest_poll(config:&ExchangeClientConfig,publisher:&mut RedisPublisher){loop{for symbol in &config.symbols{match fetch_open_interest(config,symbol).await{Ok(value)=>{if let Err(error)=publisher.publish_open_interest(&value).await{warn!(%error,%symbol,"failed to publish open interest");}},Err(error)=>warn!(%error,%symbol,"failed to fetch open interest"),}}sleep(Duration::from_secs(30)).await;}}
