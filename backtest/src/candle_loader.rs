//! Loads historical candle data from a CSV file (via Polars) or Binance's
//! public REST API.

use std::path::Path;

use polars::prelude::*;
use serde::Deserialize;
use shared::Candle;

use crate::error::{BacktestError, Result};

/// Loads candles from a CSV file with `open_time,open,high,low,close,volume,close_time`
/// columns (extra columns are ignored). `symbol` and `interval` are not
/// read from the file since historical CSV exports typically omit them.
pub fn load_candles_csv(path: impl AsRef<Path>, symbol: &str, interval: &str) -> Result<Vec<Candle>> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(path.as_ref().to_path_buf()))?
        .finish()?;

    let open_time = column_i64(&df, "open_time")?;
    let close_time = column_i64(&df, "close_time")?;
    let open = column_f64(&df, "open")?;
    let high = column_f64(&df, "high")?;
    let low = column_f64(&df, "low")?;
    let close = column_f64(&df, "close")?;
    let volume = column_f64(&df, "volume")?;

    let rows = open_time.len();
    let mut candles = Vec::with_capacity(rows);
    for i in 0..rows {
        candles.push(Candle {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
            open_time: open_time[i],
            close_time: close_time[i],
            open: open[i],
            high: high[i],
            low: low[i],
            close: close[i],
            volume: volume[i],
        });
    }
    Ok(candles)
}

fn column_f64(df: &DataFrame, name: &str) -> Result<Vec<f64>> {
    let column = df
        .column(name)
        .map_err(|_| BacktestError::MissingColumn(name_static(name)))?;
    Ok(column.cast(&DataType::Float64)?.f64()?.into_no_null_iter().collect())
}

fn column_i64(df: &DataFrame, name: &str) -> Result<Vec<i64>> {
    let column = df
        .column(name)
        .map_err(|_| BacktestError::MissingColumn(name_static(name)))?;
    Ok(column.cast(&DataType::Int64)?.i64()?.into_no_null_iter().collect())
}

/// Column names used here are all `'static` string literals, so this just
/// recovers that fact for the error type rather than allocating.
fn name_static(name: &str) -> &'static str {
    match name {
        "open_time" => "open_time",
        "close_time" => "close_time",
        "open" => "open",
        "high" => "high",
        "low" => "low",
        "close" => "close",
        "volume" => "volume",
        _ => "unknown",
    }
}

/// Binance kline REST response row. Only the fields Boldtrace uses are
/// named; the remaining trailing fields (quote volume, trade count, taker
/// volumes, and an unused element) still need to be deserialized
/// positionally so `serde` accepts the full 12-element array, even though
/// nothing here reads them.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawKline(
    i64,    // open time
    String, // open
    String, // high
    String, // low
    String, // close
    String, // volume
    i64,    // close time
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
);

/// Fetches historical klines from Binance's public REST API
/// (`GET /api/v3/klines`). `limit` is capped at 1000 by the API itself.
pub async fn fetch_candles_binance(
    symbol: &str,
    interval: &str,
    limit: u32,
) -> Result<Vec<Candle>> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={symbol}&interval={interval}&limit={limit}"
    );
    let raw: Vec<RawKline> = reqwest::get(url).await?.json().await?;
    Ok(raw
        .into_iter()
        .filter_map(|k| {
            Some(Candle {
                symbol: symbol.to_string(),
                interval: interval.to_string(),
                open_time: k.0,
                close_time: k.6,
                open: k.1.parse().ok()?,
                high: k.2.parse().ok()?,
                low: k.3.parse().ok()?,
                close: k.4.parse().ok()?,
                volume: k.5.parse().ok()?,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_candles_from_csv() {
        let mut file = tempfile_with_content(
            "open_time,open,high,low,close,volume,close_time\n\
             0,100.0,101.0,99.0,100.5,10.0,59999\n\
             60000,100.5,102.0,100.0,101.5,12.0,119999\n",
        );
        file.flush().unwrap();
        let candles = load_candles_csv(file.path(), "BTCUSDT", "1m").unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].symbol, "BTCUSDT");
        assert_eq!(candles[0].open_time, 0);
        assert_eq!(candles[1].close, 101.5);
    }

    fn tempfile_with_content(content: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }
}
