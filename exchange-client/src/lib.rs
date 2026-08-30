//! WebSocket and REST connectivity to supported exchange market data.

mod backoff;
mod binance_messages;
mod bybit;
mod candle_stream;
mod config;
mod discovery;
mod error;
mod funding_stream;
mod open_interest;
mod redis_publisher;

pub use config::{ExchangeClientConfig, ExchangeProvider};
pub use discovery::fetch_perpetual_symbols;
pub use error::{ExchangeClientError, Result};
pub use funding_stream::fetch_funding_rate_history;
pub use open_interest::fetch_open_interest;
pub use redis_publisher::RedisPublisher;

// Binance's combined-stream WebSocket endpoint rejects the request with
// HTTP 414 once the `?streams=` query string gets too long. At the old
// value of 900 (450 symbols x 2 streams/connection), a full "ALL" symbol
// universe (500+ USDT perpetuals, some with long meme-coin names like
// `1000000mogusdt`) reliably triggered a permanent 414 reconnect loop on
// both the candle and funding/depth connections. 160 keeps each combined
// URL comfortably under a few KB even for the longest symbol names.
const MAX_BINANCE_STREAMS_PER_CONNECTION: usize = 160;
const MAX_BYBIT_SYMBOLS_PER_CONNECTION: usize = 50;

fn chunk_symbols(symbols: &[String], max_symbols: usize) -> Vec<Vec<String>> {
    symbols.chunks(max_symbols.max(1)).map(<[String]>::to_vec).collect()
}

async fn run_binance(config: ExchangeClientConfig) -> Result<()> {
    let mut handles = Vec::new();
    let candle_chunk = (MAX_BINANCE_STREAMS_PER_CONNECTION / 2).max(1);
    for chunk in chunk_symbols(&config.symbols, candle_chunk) {
        let mut publisher = RedisPublisher::connect(&config.redis_url).await?;
        let chunk_config = config.with_symbols(chunk);
        handles.push(tokio::spawn(async move {
            candle_stream::run_candle_stream(&chunk_config, &mut publisher).await
        }));
    }
    let futures_chunk = (MAX_BINANCE_STREAMS_PER_CONNECTION / 2).max(1);
    for chunk in chunk_symbols(&config.symbols, futures_chunk) {
        let mut publisher = RedisPublisher::connect(&config.redis_url).await?;
        let chunk_config = config.with_symbols(chunk);
        handles.push(tokio::spawn(async move {
            funding_stream::run_funding_stream(&chunk_config, &mut publisher).await
        }));
    }
    let mut oi_publisher = RedisPublisher::connect(&config.redis_url).await?;
    let oi_config = config.clone();
    handles.push(tokio::spawn(async move {
        open_interest::run_open_interest_poll(&oi_config, &mut oi_publisher).await;
        Ok(())
    }));
    for handle in handles {
        handle.await.map_err(|err| ExchangeClientError::InvalidMarketData(format!("exchange task join failed: {err}")))??;
    }
    Ok(())
}

async fn run_bybit(config: ExchangeClientConfig) -> Result<()> {
    let mut handles = Vec::new();
    for chunk in chunk_symbols(&config.symbols, MAX_BYBIT_SYMBOLS_PER_CONNECTION) {
        let mut publisher = RedisPublisher::connect(&config.redis_url).await?;
        let chunk_config = config.with_symbols(chunk);
        handles.push(tokio::spawn(async move {
            bybit::run_bybit_stream(&chunk_config, &mut publisher).await
        }));
    }
    for handle in handles {
        handle.await.map_err(|err| ExchangeClientError::InvalidMarketData(format!("Bybit task join failed: {err}")))??;
    }
    Ok(())
}

pub async fn run(config: ExchangeClientConfig) -> Result<()> {
    match config.provider {
        ExchangeProvider::Binance => run_binance(config).await,
        ExchangeProvider::Bybit => run_bybit(config).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_preserve_all_symbols() {
        let symbols: Vec<String> = (0..123).map(|i| format!("SYM{i}")).collect();
        let chunks = chunk_symbols(&symbols, 50);
        assert!(chunks.iter().all(|chunk| chunk.len() <= 50));
        assert_eq!(chunks.iter().map(Vec::len).sum::<usize>(), symbols.len());
    }

    #[test]
    fn small_lists_produce_one_chunk() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        assert_eq!(chunk_symbols(&symbols, 50), vec![symbols]);
    }
}
