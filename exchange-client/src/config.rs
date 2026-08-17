//! Runtime configuration for the exchange-client crate.

/// Configuration for connecting to Binance and publishing to Redis.
#[derive(Debug, Clone)]
pub struct ExchangeClientConfig {
    /// Trading symbol to subscribe to, e.g. `BTCUSDT`.
    pub symbol: String,
    /// Redis connection URL, e.g. `redis://127.0.0.1:6379`.
    pub redis_url: String,
    /// Base URL for the Binance spot combined WebSocket stream.
    pub spot_ws_base: String,
    /// Base URL for the Binance USDT-margined futures WebSocket stream
    /// (used for funding rate, which does not exist on the spot market).
    pub futures_ws_base: String,
    /// Base URL for the Binance USDT-margined futures REST API (used as a
    /// fallback to fetch funding rate history).
    pub futures_rest_base: String,
}

impl ExchangeClientConfig {
    /// Builds a config for `symbol` using Binance's public endpoints and
    /// the given Redis URL.
    pub fn new(symbol: impl Into<String>, redis_url: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            redis_url: redis_url.into(),
            spot_ws_base: "wss://stream.binance.com:9443/stream".to_string(),
            futures_ws_base: "wss://fstream.binance.com/stream".to_string(),
            futures_rest_base: "https://fapi.binance.com".to_string(),
        }
    }

    /// Reads configuration from environment variables, falling back to
    /// `BTCUSDT` and a local Redis instance when unset.
    pub fn from_env() -> Self {
        let symbol = std::env::var("EXCHANGE_SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        Self::new(symbol, redis_url)
    }

    /// Lowercase symbol, as used in Binance stream names.
    pub fn symbol_lower(&self) -> String {
        self.symbol.to_lowercase()
    }
}
