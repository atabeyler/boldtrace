//! Runtime configuration for the exchange-client crate.

use crate::discovery::fetch_perpetual_symbols;
use crate::error::Result;

/// Sentinel value for `EXCHANGE_SYMBOLS` that means "discover and track
/// every currently trading USDT perpetual future", rather than a fixed
/// list that goes stale as Binance lists or delists symbols.
const ALL_SYMBOLS: &str = "ALL";

/// Configuration for connecting to Binance and publishing to Redis. A
/// single process tracks any number of symbols concurrently, spread across
/// as many WebSocket connections as needed to stay under Binance's
/// per-connection stream limit.
#[derive(Debug, Clone)]
pub struct ExchangeClientConfig {
    /// Trading symbols to subscribe to, e.g. `["BTCUSDT", "ETHUSDT"]`. A
    /// single `"ALL"` entry means "resolve via `resolve_symbols` before
    /// use" — callers that skip that step would otherwise try to open a
    /// literal `ALL` stream, which fails loudly and obviously.
    pub symbols: Vec<String>,
    /// Redis connection URL, e.g. `redis://127.0.0.1:6379`.
    pub redis_url: String,
    /// Base URL for the Binance USDT-margined futures WebSocket stream.
    /// Candles, order book depth and funding rate are all sourced from the
    /// futures market rather than spot: funding rate and open interest only
    /// exist there, and this product's signals are defined against
    /// perpetual futures, so keeping every data source on the same market
    /// avoids subtle spot/futures divergence and guarantees every
    /// discovered symbol actually has all four data types.
    pub futures_ws_base: String,
    /// Base URL for the Binance USDT-margined futures REST API (used to
    /// discover the tradable symbol universe, fetch funding rate history,
    /// and poll open interest).
    pub futures_rest_base: String,
}

impl ExchangeClientConfig {
    /// Builds a config for `symbols` using Binance's public endpoints and
    /// the given Redis URL.
    pub fn new(symbols: Vec<String>, redis_url: impl Into<String>) -> Self {
        Self {
            symbols,
            redis_url: redis_url.into(),
            futures_ws_base: "wss://fstream.binance.com/stream".to_string(),
            futures_rest_base: "https://fapi.binance.com".to_string(),
        }
    }

    /// Reads configuration from environment variables. `EXCHANGE_SYMBOLS`
    /// (comma-separated, e.g. `BTCUSDT,ETHUSDT,SOLUSDT`, or `ALL` to track
    /// every USDT perpetual future) is preferred; the older single-symbol
    /// `EXCHANGE_SYMBOL` is still honored for deployments that haven't
    /// switched over. Falls back to `BTCUSDT` and a local Redis instance
    /// when unset.
    pub fn from_env() -> Self {
        let symbols = std::env::var("EXCHANGE_SYMBOLS")
            .or_else(|_| std::env::var("EXCHANGE_SYMBOL"))
            .unwrap_or_else(|_| "BTCUSDT".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        Self::new(parse_symbols(&symbols), redis_url)
    }

    /// Returns a clone with `symbols` replaced, for splitting one config
    /// into per-connection chunks.
    pub fn with_symbols(&self, symbols: Vec<String>) -> Self {
        Self { symbols, ..self.clone() }
    }

    /// If `symbols` is the `ALL` sentinel, replaces it with the live list
    /// of tradable USDT perpetual futures from Binance. A no-op otherwise.
    pub async fn resolve_symbols(self) -> Result<Self> {
        if self.symbols == [ALL_SYMBOLS] {
            let symbols = fetch_perpetual_symbols(&self).await?;
            tracing::info!(count = symbols.len(), "discovered all USDT perpetual futures");
            Ok(self.with_symbols(symbols))
        } else {
            Ok(self)
        }
    }
}

/// Splits, trims, uppercases and dedupes a comma-separated symbol list,
/// preserving first-seen order. A bare `ALL` (any case) resolves to the
/// single-element sentinel list regardless of what else was written.
fn parse_symbols(raw: &str) -> Vec<String> {
    if raw.trim().eq_ignore_ascii_case(ALL_SYMBOLS) {
        return vec![ALL_SYMBOLS.to_string()];
    }
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

    #[test]
    fn recognizes_all_sentinel_case_insensitively() {
        assert_eq!(parse_symbols("all"), vec!["ALL"]);
        assert_eq!(parse_symbols(" All "), vec!["ALL"]);
    }
}
