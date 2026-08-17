//! Exports a [`BacktestResult`] to JSON or CSV so the bot can show
//! "historical performance" to users.

use std::path::Path;

use crate::error::Result;
use crate::runner::BacktestResult;

/// Writes the full result, including every individual signal, as JSON.
pub fn export_json(result: &BacktestResult, path: impl AsRef<Path>) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Writes one row per signal as CSV; the aggregate stats
/// (`win_rate`, `average_return_pct`) are not repeated per row and should
/// be read from the JSON export or the in-memory `BacktestResult`.
pub fn export_csv(result: &BacktestResult, path: impl AsRef<Path>) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    for signal in &result.signals {
        writer.serialize(signal)?;
    }
    writer.flush()?;
    Ok(())
}
