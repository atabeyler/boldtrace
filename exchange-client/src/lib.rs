//! WebSocket and REST connectivity to exchange market data (Binance).

mod backoff;
mod binance_messages;
mod candle_stream;
mod config;
mod discovery;
mod error;
mod funding_stream;
mod open_interest;
mod redis_publisher;

pub use config::ExchangeClientConfig;
pub use discovery::fetch_perpetual_symbols;
pub use error::{ExchangeClientError, Result};
pub use funding_stream::fetch_funding_rate_history;
pub use open_interest::fetch_open_interest;
pub use redis_publisher::RedisPublisher;

/// Binance allows up to 1024 streams on one combined-stream WebSocket
/// connection. Chunk sizing stays well under that so a burst of new
/// listings (when discovering `ALL`) never risks tripping the limit.
const MAX_STREAMS_PER_CONNECTION: usize = 900;

/// Splits `symbols` into chunks small enough that `symbols_per_chunk *
/// streams_per_symbol` stays under Binance's per-connection stream limit,
/// so `run` can open one WebSocket connection per chunk.
fn chunk_symbols(symbols: &[String], streams_per_symbol: usize) -> Vec<Vec<String>> {
    let chunk_size = (MAX_STREAMS_PER_CONNECTION / streams_per_symbol.max(1)).max(1);
    symbols.chunks(chunk_size).map(<[String]>::to_vec).collect()
}

pub async fn run(config: ExchangeClientConfig) -> Result<()> {
    let mut handles = Vec::new();

    // Candles + order book depth: 3 streams/symbol (kline_1m, kline_5m, depth20).
    for chunk in chunk_symbols(&config.symbols, 3) {
        let mut publisher = RedisPublisher::connect(&config.redis_url).await?;
        let chunk_config = config.with_symbols(chunk);
        handles.push(tokio::spawn(async move {
            candle_stream::run_candle_stream(&chunk_config, &mut publisher).await
        }));
    }

    // Funding rate (mark price): 1 stream/symbol.
    for chunk in chunk_symbols(&config.symbols, 1) {
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
        let _ = handle.await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_stay_under_the_stream_limit() {
        let symbols: Vec<String> = (0..1000).map(|i| format!("SYM{i}")).collect();
        let chunks = chunk_symbols(&symbols, 3);
        assert!(chunks.iter().all(|c| c.len() * 3 <= MAX_STREAMS_PER_CONNECTION));
        assert_eq!(chunks.iter().map(Vec::len).sum::<usize>(), 1000);
    }

    #[test]
    fn small_lists_produce_one_chunk() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        assert_eq!(chunk_symbols(&symbols, 3), vec![symbols]);
    }
}
