//! Runtime configuration for the exchange-client crate.

use crate::bybit::fetch_bybit_perpetual_symbols;
use crate::discovery::fetch_perpetual_symbols;
use crate::error::Result;

const ALL_SYMBOLS: &str = "ALL";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeProvider {
    Binance,
    Bybit,
}

impl ExchangeProvider {
    fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "bybit" => Self::Bybit,
            _ => Self::Binance,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Binance => "binance",
            Self::Bybit => "bybit",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExchangeClientConfig {
    pub provider: ExchangeProvider,
    pub symbols: Vec<String>,
    pub redis_url: String,
    pub spot_ws_base: String,
    pub futures_ws_base: String,
    pub futures_rest_base: String,
    pub bybit_ws_base: String,
    pub bybit_rest_base: String,
}

impl ExchangeClientConfig {
    pub fn new(symbols: Vec<String>, redis_url: impl Into<String>) -> Self {
        Self {
            provider: ExchangeProvider::Binance,
            symbols,
            redis_url: redis_url.into(),
            spot_ws_base: "wss://stream.binance.com:9443/stream".to_string(),
            futures_ws_base: "wss://fstream.binance.com/stream".to_string(),
            futures_rest_base: "https://fapi.binance.com".to_string(),
            bybit_ws_base: "wss://stream.bybit.com/v5/public/linear".to_string(),
            bybit_rest_base: "https://api.bybit.com".to_string(),
        }
    }

    pub fn from_env() -> Self {
        let symbols = std::env::var("EXCHANGE_SYMBOLS")
            .or_else(|_| std::env::var("EXCHANGE_SYMBOL"))
            .unwrap_or_else(|_| "BTCUSDT".to_string());
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let provider = ExchangeProvider::from_env_value(
            &std::env::var("EXCHANGE_PROVIDER").unwrap_or_else(|_| "binance".to_string()),
        );
        let mut config = Self::new(parse_symbols(&symbols), redis_url);
        config.provider = provider;
        config
    }

    pub fn with_symbols(&self, symbols: Vec<String>) -> Self {
        Self { symbols, ..self.clone() }
    }

    pub async fn resolve_symbols(self) -> Result<Self> {
        if self.symbols != [ALL_SYMBOLS] {
            return Ok(self);
        }
        let symbols = match self.provider {
            ExchangeProvider::Binance => fetch_perpetual_symbols(&self).await?,
            ExchangeProvider::Bybit => fetch_bybit_perpetual_symbols(&self).await?,
        };
        tracing::info!(provider = self.provider.as_str(), count = symbols.len(), "discovered active USDT perpetual futures");
        Ok(self.with_symbols(symbols))
    }
}

fn parse_symbols(raw: &str) -> Vec<String> {
    if raw.trim().eq_ignore_ascii_case(ALL_SYMBOLS) {
        return vec![ALL_SYMBOLS.to_string()];
    }
    let mut seen = std::collections::HashSet::new();
    raw.split(',')
        .map(|symbol| symbol.trim().to_uppercase())
        .filter(|symbol| !symbol.is_empty())
        .filter(|symbol| seen.insert(symbol.clone()))
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

    #[test]
    fn provider_defaults_unknown_values_to_binance() {
        assert_eq!(ExchangeProvider::from_env_value("BYBIT"), ExchangeProvider::Bybit);
        assert_eq!(ExchangeProvider::from_env_value("anything"), ExchangeProvider::Binance);
    }
}
