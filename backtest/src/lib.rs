//! Historical backtesting of the score engine against past market data.
mod candle_loader;
mod error;
mod export;
mod frame_loader;
mod metrics;
mod runner;
pub use candle_loader::{fetch_candles_binance, load_candles_csv};
pub use error::{BacktestError, Result};
pub use export::{export_csv, export_json};
pub use frame_loader::load_market_frames_json;
pub use metrics::{performance_metrics, PerformanceMetrics};
pub use runner::{
    run_backtest, run_backtest_with_costs, run_full_backtest, BacktestCosts, BacktestResult,
    BacktestSide, BacktestSignal, HistoricalMarketFrame,
};
