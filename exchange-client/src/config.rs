//! Runtime configuration for the exchange-client crate.

use crate::discovery::fetch_perpetual_symbols;
use crate::error::Result;

const ALL_SYMBOLS: &str = "ALL";

#[derive(Debug, Clone)]
pub struct ExchangeClientConfig {
    pub symbols: Vec<String>,
    pub redis_url: String,
    /// Spot combined stream is used for klines.  BOLDTRACE deliberately
    /// keeps candles on this proven feed while perpetual-market depth,
    /// funding and open interest are sourced from futures endpoints.
    pub spot_ws_base: String,
    /// Binance USDⓈ-M futures combined stream for mark-price/funding and
    /// perpetual order-book depth.
    pub futures_ws_base: String,
    /// Binance USDⓈ-M REST API for symbol discovery, funding history and
    /// open-interest polling.
    pub futures_rest_base: String,
}

impl ExchangeClientConfig {
    pub fn new(symbols: Vec<String>, redis_url: impl Into<String>) -> Self {
        Self {
            symbols,
            redis_url: redis_url.into(),
            spot_ws_base: "wss://stream.binance.com:9443/stream".to_string(),
            futures_ws_base: "wss://fstream.binance.com/stream".to_string(),
            futures_rest_base: "https://fapi.binance.com".to_string(),
        }
    }

    pub fn from_env() -> Self {
        let symbols = std::env::var("EXCHANGE_SYMBOLS")
            .or_else(|_| std::env::var("EXCHANGE_SYMBOL"))
            .unwrap_or_else(|_| "BTCUSDT".to_string());
        let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        Self::new(parse_symbols(&symbols), redis_url)
    }

    pub fn with_symbols(&self, symbols: Vec<String>) -> Self {
        Self { symbols, ..self.clone() }
    }

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
        assert_eq!(parse_symbols(" btcusdt, ETHUSDT,btcusdt , solusdt"), vec!["BTCUSDT", "ETHUSDT", "SOLUSDT"]);
    }
    #[test]
    fn recognizes_all_sentinel_case_insensitively() {
        assert_eq!(parse_symbols("all"), vec!["ALL"]);
        assert_eq!(parse_symbols(" All "), vec!["ALL"]);
    }
}
