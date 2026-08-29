//! CLI entry point for candle-only or full-market-frame backtests.
#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    let symbol = std::env::var("BACKTEST_SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
    let interval = std::env::var("BACKTEST_INTERVAL").unwrap_or_else(|_| "1h".to_string());
    let score_threshold: f64 = std::env::var("BACKTEST_SCORE_THRESHOLD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(70.0);
    let lookahead_hours: i64 = std::env::var("BACKTEST_LOOKAHEAD_HOURS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4);
    let output = std::env::var("BACKTEST_OUTPUT").unwrap_or_else(|_| "backtest_result.json".to_string());
    let costs = backtest::BacktestCosts {
        round_trip_fee_pct: std::env::var("BACKTEST_ROUND_TRIP_FEE_PCT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0),
        slippage_pct: std::env::var("BACKTEST_SLIPPAGE_PCT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0),
        funding_pct: std::env::var("BACKTEST_FUNDING_PCT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0),
    };
    let weights = score_engine::ScoreWeights::from_env();

    let result = if let Ok(path) = std::env::var("BACKTEST_FRAMES_JSON") {
        let frames = match backtest::load_market_frames_json(path) {
            Ok(frames) => frames,
            Err(err) => {
                tracing::error!(error=%err, "failed to load archived full-market frames");
                std::process::exit(1);
            }
        };
        backtest::run_full_backtest(&frames, &weights, score_threshold, lookahead_hours, costs)
    } else {
        let candles = match std::env::var("BACKTEST_CSV") {
            Ok(path) => backtest::load_candles_csv(&path, &symbol, &interval),
            Err(_) => backtest::fetch_candles_binance(&symbol, &interval, 1000).await,
        };
        let candles = match candles {
            Ok(candles) => candles,
            Err(err) => {
                tracing::error!(error = %err, "failed to load historical candles");
                std::process::exit(1);
            }
        };
        backtest::run_backtest_with_costs(&candles, &weights, score_threshold, lookahead_hours, costs)
    };

    let metrics = backtest::performance_metrics(&result.signals);
    tracing::info!(
        scope=%result.scope,
        total_signals=result.total_signals,
        long_signals=result.long_signals,
        short_signals=result.short_signals,
        no_trade_points=result.no_trade_points,
        win_rate=result.win_rate,
        average_net_return_pct=result.average_return_pct,
        profit_factor=metrics.profit_factor,
        max_drawdown_pct=metrics.max_drawdown_pct,
        expectancy_pct=metrics.expectancy_pct,
        payoff_ratio=metrics.payoff_ratio,
        "backtest complete"
    );
    if result.costs == backtest::BacktestCosts::default() {
        tracing::warn!("backtest costs are zero; set fee/slippage/funding env vars before treating returns as execution-realistic");
    }
    if result.scope == "candle-only" {
        tracing::warn!("candle-only result does not validate funding, open-interest, futures depth or full risk gating; use BACKTEST_FRAMES_JSON for full-market-frame validation");
    }
    if let Err(err) = backtest::export_json(&result, &output) {
        tracing::error!(error=%err, "failed to export backtest result");
        std::process::exit(1);
    }
}
