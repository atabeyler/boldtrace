//! Loader for archived full-market frames used by the full-engine backtest.
//! Input is a JSON array of `HistoricalMarketFrame` objects. Missing market
//! sources must be encoded as null; the risk engine will downgrade quality
//! rather than receiving synthetic neutral data.

use crate::error::Result;
use crate::HistoricalMarketFrame;
use std::path::Path;

pub fn load_market_frames_json(path: impl AsRef<Path>) -> Result<Vec<HistoricalMarketFrame>> {
    let raw = std::fs::read_to_string(path)?;
    let frames: Vec<HistoricalMarketFrame> = serde_json::from_str(&raw)?;
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::BacktestError;

    #[test]
    fn invalid_json_is_rejected() {
        let path = std::env::temp_dir().join("boldtrace-invalid-market-frames.json");
        std::fs::write(&path, "not-json").unwrap();
        let result = load_market_frames_json(&path);
        let _ = std::fs::remove_file(path);
        assert!(matches!(result, Err(BacktestError::Json(_))));
    }
}
