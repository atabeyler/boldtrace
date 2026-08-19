//! Discovers the live set of USDT-margined perpetual futures symbols from
//! Binance, so `EXCHANGE_SYMBOLS=ALL` tracks the real tradable universe
//! instead of a list that goes stale as symbols get listed or delisted.

use serde::Deserialize;

use crate::config::ExchangeClientConfig;
use crate::error::Result;

#[derive(Debug, Deserialize)]
struct ExchangeInfo {
    symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Deserialize)]
struct SymbolInfo {
    symbol: String,
    status: String,
    #[serde(rename = "contractType")]
    contract_type: String,
    #[serde(rename = "quoteAsset")]
    quote_asset: String,
}

/// Fetches every currently trading USDT-margined perpetual future from
/// Binance's futures exchange info endpoint.
pub async fn fetch_perpetual_symbols(config: &ExchangeClientConfig) -> Result<Vec<String>> {
    let url = format!("{}/fapi/v1/exchangeInfo", config.futures_rest_base);
    let info: ExchangeInfo = reqwest::get(url).await?.json().await?;
    let mut symbols: Vec<String> = info
        .symbols
        .into_iter()
        .filter(|s| s.status == "TRADING" && s.contract_type == "PERPETUAL" && s.quote_asset == "USDT")
        .map(|s| s.symbol)
        .collect();
    symbols.sort();
    Ok(symbols)
}
