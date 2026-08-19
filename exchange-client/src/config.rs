//! Runtime configuration for the exchange-client crate.

/// Configuration for connecting to Binance and publishing to Redis. A
/// single process tracks any number of symbols concurrently over one
/// combined WebSocket connection per stream type.
#[derive(Debug, Clone)]
pub struct ExchangeClientConfig {
    /// Trading symbols to subscribe to, e.g. `["BTCUSDT", "ETHUSDT"]`.
    pub symbols: Vec<String>,
    /// Redis connection URL, e.g. `redis://127.0.0.1:6379`.
    pub redis_url: String,
    /// Base URL for the Binance spot combined WebSocket stream.
    pub spot_ws_base: String,
    /// Base URL for the Binance USDT-margined futures WebSocket stream
    /// (used for funding rate, which does not exist on the spot market).
    pub futures_ws_base: String,
    /// Base URL for the Binance USDT-margined futures REST API (used as a
    /// fallback to fetch funding rate history, and to poll open interest).
    pub futures_rest_base: String,
}

impl ExchangeClientConfig {
    /// Builds a config for `symbols` using Binance's public endpoints and
    /// the given Redis URL.
    pub fn new(symbols: Vec<String>, redis_url: impl Into<String>) -> Self {
        Self {
            symbols,
            redis_url: redis_url.into(),
            spot_ws_base: "wss://stream.binance.com:9443/stream".to_string(),
            futures_ws_base: "wss://fstream.binance.com/stream".to_string(),
            futures_rest_base: "https://fapi.binance.com".to_string(),
        }
    }

    /// Reads configuration from environment variables. `EXCHANGE_SYMBOLS`
    /// (comma-separated, e.g. `BTCUSDT,ETHUSDT,SOLUSDT`) is preferred; the
    /// older single-symbol `EXCHANGE_SYMBOL` is still honored for
    /// deployments that haven't switched over. Falls back to `BTCUSDT` and
    /// a local Redis instance when unset.
    pub fn from_env() -> Self {
        let symbols = std::env::var("EXCHANGE_SYMBOLS")
            .or_else(|_| std::env::var("EXCHANGE_SYMBOL"))
            .unwrap_or_else(|_| "BTCUSDT".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        Self::new(parse_symbols(&symbols), redis_url)
    }
}

/// Splits, trims, uppercases and dedupes a comma-separated symbol list,
/// preserving first-seen order.
fn parse_symbols(raw: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .filter(|s| seen.insert(s.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_dedupes_symbol_list() {
        assert_eq!(
            parse_symbols(" btcusdt, ETHUSDT,btcusdt , solusdt"),
            vec!["BTCUSDT", "ETHUSDT", "SOLUSDT"]
        );
    }
}
