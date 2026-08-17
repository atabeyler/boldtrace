//! Error type for the backtest crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BacktestError {
    #[error("failed to read historical data: {0}")]
    Polars(#[from] polars::error::PolarsError),

    #[error("http request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    #[error("missing required CSV column: {0}")]
    MissingColumn(&'static str),
}

pub type Result<T> = std::result::Result<T, BacktestError>;
