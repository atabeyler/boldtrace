//! Runs `score-engine` over historical candles and measures how price
//! behaved after each score-threshold crossing.

use serde::Serialize;
use shared::{Candle, FundingRate, OrderBookSnapshot};

/// Number of trailing candles fed into `score-engine` for each score
/// computation; large enough to cover the RSI/volume-anomaly lookback
/// windows those signals need.
const SCORE_WINDOW: usize = 50;

/// A single point in time where the composite score crossed the
/// configured threshold, together with the price outcome `lookahead`
/// candles later.
#[derive(Debug, Clone, Serialize)]
pub struct BacktestSignal {
    pub symbol: String,
    pub timestamp: i64,
    pub score: f64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub return_pct: f64,
    pub win: bool,
}

/// Aggregate statistics produced by [`run_backtest`].
#[derive(Debug, Clone, Serialize)]
pub struct BacktestResult {
    pub symbol: String,
    pub interval: String,
    pub score_threshold: f64,
    pub lookahead_hours: i64,
    pub total_signals: usize,
    pub win_rate: f64,
    pub average_return_pct: f64,
    pub signals: Vec<BacktestSignal>,
}

/// Runs `score-engine::calculate` over a sliding window of `candles`,
/// recording every point where the score reaches `score_threshold` and
/// the price outcome `lookahead_hours` later.
///
/// Historical candle data alone does not include order book depth or
/// funding rate history, so both are supplied as neutral inputs
/// (`funding_rate = 0`, an empty order book) for every window; only the
/// volume-anomaly and RSI-divergence signals are backtested against real
/// data unless the caller zeroes those weights out via `weights`.
pub fn run_backtest(
    candles: &[Candle],
    weights: &score_engine::ScoreWeights,
    score_threshold: f64,
    lookahead_hours: i64,
) -> BacktestResult {
    let symbol = candles
        .first()
        .map(|c| c.symbol.clone())
        .unwrap_or_default();
    let interval = candles
        .first()
        .map(|c| c.interval.clone())
        .unwrap_or_default();

    let lookahead_millis = lookahead_hours * 3_600_000;
    let mut signals = Vec::new();

    for i in 0..candles.len() {
        let window_start = i.saturating_sub(SCORE_WINDOW - 1);
        let window = &candles[window_start..=i];
        let current = &candles[i];

        let input = score_engine::ScoreInput {
            candles: window.to_vec(),
            order_book: OrderBookSnapshot {
                symbol: current.symbol.clone(),
                timestamp: current.close_time,
                bids: Vec::new(),
                asks: Vec::new(),
            },
            funding_rate: FundingRate {
                symbol: current.symbol.clone(),
                timestamp: current.close_time,
                rate: 0.0,
            },
        };
        let score = score_engine::calculate(&input, weights);
        if score.value < score_threshold {
            continue;
        }

        let target_time = current.close_time + lookahead_millis;
        let Some(exit_candle) = candles[i + 1..]
            .iter()
            .find(|candle| candle.close_time >= target_time)
        else {
            continue;
        };

        let entry_price = current.close;
        let exit_price = exit_candle.close;
        let return_pct = (exit_price - entry_price) / entry_price * 100.0;

        signals.push(BacktestSignal {
            symbol: current.symbol.clone(),
            timestamp: current.close_time,
            score: score.value,
            entry_price,
            exit_price,
            return_pct,
            win: return_pct > 0.0,
        });
    }

    let total_signals = signals.len();
    let win_rate = if total_signals == 0 {
        0.0
    } else {
        signals.iter().filter(|s| s.win).count() as f64 / total_signals as f64 * 100.0
    };
    let average_return_pct = if total_signals == 0 {
        0.0
    } else {
        signals.iter().map(|s| s.return_pct).sum::<f64>() / total_signals as f64
    };

    BacktestResult {
        symbol,
        interval,
        score_threshold,
        lookahead_hours,
        total_signals,
        win_rate,
        average_return_pct,
        signals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use score_engine::ScoreWeights;

    fn candle(i: i64, close: f64, volume: f64) -> Candle {
        Candle {
            symbol: "BTCUSDT".to_string(),
            interval: "1h".to_string(),
            open_time: i * 3_600_000,
            close_time: i * 3_600_000 + 3_600_000,
            open: close,
            high: close,
            low: close,
            close,
            volume,
        }
    }

    #[test]
    fn empty_input_produces_empty_result() {
        let result = run_backtest(&[], &ScoreWeights::default(), 70.0, 4);
        assert_eq!(result.total_signals, 0);
        assert_eq!(result.win_rate, 0.0);
    }

    #[test]
    fn detects_a_volume_spike_signal_and_measures_its_outcome() {
        // Flat volume baseline, then one huge spike candle followed by
        // enough future candles to measure the 4h-later outcome.
        let mut candles: Vec<Candle> = (0..30).map(|i| candle(i, 100.0, 100.0)).collect();
        candles.push(candle(30, 100.0, 100_000.0)); // spike
        candles.extend((31..40).map(|i| candle(i, 110.0, 100.0)));

        let weights = ScoreWeights {
            volume_anomaly: 1.0,
            funding_extreme: 0.0,
            order_book_imbalance: 0.0,
            rsi_divergence: 0.0,
        };
        let result = run_backtest(&candles, &weights, 70.0, 4);
        assert_eq!(result.total_signals, 1);
        let signal = &result.signals[0];
        assert!(signal.score >= 70.0);
        assert!(signal.win);
        assert!(signal.return_pct > 0.0);
    }
}
