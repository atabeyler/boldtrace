//! Historical backtesting of the score engine against past market data.

mod candle_loader;
mod error;
mod export;
mod runner;

pub use candle_loader::{fetch_candles_binance, load_candles_csv};
pub use error::{BacktestError, Result};
pub use export::{export_csv, export_json};
pub use runner::{run_backtest, BacktestResult, BacktestSignal};
