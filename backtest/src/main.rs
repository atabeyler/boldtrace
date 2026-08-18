//! CLI entry point for running a backtest and exporting the results.
#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    let symbol = std::env::var("BACKTEST_SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
    let interval = std::env::var("BACKTEST_INTERVAL").unwrap_or_else(|_| "1h".to_string());
    let score_threshold: f64 = std::env::var("BACKTEST_SCORE_THRESHOLD").ok().and_then(|v| v.parse().ok()).unwrap_or(70.0);
    let lookahead_hours: i64 = std::env::var("BACKTEST_LOOKAHEAD_HOURS").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let output = std::env::var("BACKTEST_OUTPUT").unwrap_or_else(|_| "backtest_result.json".to_string());
    let candles = match std::env::var("BACKTEST_CSV") { Ok(path) => backtest::load_candles_csv(&path, &symbol, &interval), Err(_) => backtest::fetch_candles_binance(&symbol, &interval, 1000).await };
    let candles = match candles { Ok(candles) => candles, Err(err) => { tracing::error!(error = %err, "failed to load historical candles"); std::process::exit(1); } };
    let weights = score_engine::ScoreWeights::from_env();
    let result = backtest::run_backtest(&candles, &weights, score_threshold, lookahead_hours);
    let metrics = backtest::performance_metrics(&result.signals);
    tracing::info!(total_signals=result.total_signals,win_rate=result.win_rate,average_return_pct=result.average_return_pct,profit_factor=metrics.profit_factor,max_drawdown_pct=metrics.max_drawdown_pct,expectancy_pct=metrics.expectancy_pct,payoff_ratio=metrics.payoff_ratio,"backtest complete");
    if let Err(err)=backtest::export_json(&result,&output){tracing::error!(error=%err,"failed to export backtest result");std::process::exit(1);}
}
