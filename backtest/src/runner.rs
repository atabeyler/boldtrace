//! Historical validation for BOLDTRACE market decisions.
//!
//! Candle-only runs cannot recreate funding, open-interest or order-book
//! state and are labelled accordingly. A full-frame runner accepts archived
//! observations for those sources so missing data is never fabricated.

use serde::{Deserialize, Serialize};
use shared::{Candle, FundingRate, OpenInterest, OrderBookSnapshot};
use std::collections::VecDeque;

const SCORE_WINDOW: usize = 50;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BacktestSide {
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct BacktestCosts {
    pub round_trip_fee_pct: f64,
    pub slippage_pct: f64,
    /// Positive = cost paid, negative = funding received.
    pub funding_pct: f64,
}

impl Default for BacktestCosts {
    fn default() -> Self {
        Self {
            round_trip_fee_pct: 0.0,
            slippage_pct: 0.0,
            funding_pct: 0.0,
        }
    }
}

impl BacktestCosts {
    fn total(self) -> f64 {
        self.round_trip_fee_pct.max(0.0) + self.slippage_pct.max(0.0) + self.funding_pct
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestSignal {
    pub symbol: String,
    pub timestamp: i64,
    pub score: f64,
    pub side: BacktestSide,
    pub entry_price: f64,
    pub exit_price: f64,
    pub gross_market_return_pct: f64,
    pub gross_directional_return_pct: f64,
    /// Net directional return after explicit costs.
    pub return_pct: f64,
    pub win: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestResult {
    pub symbol: String,
    pub interval: String,
    pub scope: String,
    pub score_threshold: f64,
    pub short_threshold: f64,
    pub lookahead_hours: i64,
    pub costs: BacktestCosts,
    pub total_signals: usize,
    pub long_signals: usize,
    pub short_signals: usize,
    pub no_trade_points: usize,
    pub win_rate: f64,
    pub average_return_pct: f64,
    pub signals: Vec<BacktestSignal>,
}

/// One timestamp-aligned archive frame for full-engine historical validation.
/// Sources that were genuinely unavailable at the time must be `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMarketFrame {
    pub candle: Candle,
    pub order_book: Option<OrderBookSnapshot>,
    pub funding_rate: Option<FundingRate>,
    pub open_interest: Option<OpenInterest>,
}

struct ResultMeta {
    symbol: String,
    interval: String,
    scope: &'static str,
    score_threshold: f64,
    lookahead_hours: i64,
    costs: BacktestCosts,
    no_trade_points: usize,
}

fn side_from_score(score: f64, long_threshold: f64) -> Option<BacktestSide> {
    let short_threshold = 100.0 - long_threshold;
    if score >= long_threshold {
        Some(BacktestSide::Long)
    } else if score <= short_threshold {
        Some(BacktestSide::Short)
    } else {
        None
    }
}

fn result_from_signals(meta: ResultMeta, signals: Vec<BacktestSignal>) -> BacktestResult {
    let total_signals = signals.len();
    let long_signals = signals
        .iter()
        .filter(|signal| signal.side == BacktestSide::Long)
        .count();
    let short_signals = signals
        .iter()
        .filter(|signal| signal.side == BacktestSide::Short)
        .count();
    let win_rate = if total_signals == 0 {
        0.0
    } else {
        signals.iter().filter(|signal| signal.win).count() as f64 / total_signals as f64 * 100.0
    };
    let average_return_pct = if total_signals == 0 {
        0.0
    } else {
        signals.iter().map(|signal| signal.return_pct).sum::<f64>() / total_signals as f64
    };

    BacktestResult {
        symbol: meta.symbol,
        interval: meta.interval,
        scope: meta.scope.into(),
        score_threshold: meta.score_threshold,
        short_threshold: 100.0 - meta.score_threshold,
        lookahead_hours: meta.lookahead_hours,
        costs: meta.costs,
        total_signals,
        long_signals,
        short_signals,
        no_trade_points: meta.no_trade_points,
        win_rate,
        average_return_pct,
        signals,
    }
}

fn record_signal(
    candles: &[Candle],
    index: usize,
    score: f64,
    side: BacktestSide,
    lookahead_hours: i64,
    costs: BacktestCosts,
) -> Option<BacktestSignal> {
    // The signal only exists after candle `index` closes. The next candle's
    // open is the earliest execution price available without look-ahead bias.
    let current = candles.get(index)?;
    let entry_candle = candles.get(index + 1)?;
    let target_time = current.close_time + lookahead_hours * 3_600_000;
    let exit_candle = candles[index + 1..]
        .iter()
        .find(|candle| candle.close_time >= target_time)?;
    if entry_candle.open <= 0.0 || exit_candle.close <= 0.0 {
        return None;
    }

    let gross_market_return_pct = (exit_candle.close / entry_candle.open - 1.0) * 100.0;
    let direction = if side == BacktestSide::Long { 1.0 } else { -1.0 };
    let gross_directional_return_pct = gross_market_return_pct * direction;
    let net_return_pct = gross_directional_return_pct - costs.total();

    Some(BacktestSignal {
        symbol: current.symbol.clone(),
        timestamp: current.close_time,
        score,
        side,
        entry_price: entry_candle.open,
        exit_price: exit_candle.close,
        gross_market_return_pct,
        gross_directional_return_pct,
        return_pct: net_return_pct,
        win: net_return_pct > 0.0,
    })
}

pub fn run_backtest(
    candles: &[Candle],
    weights: &score_engine::ScoreWeights,
    score_threshold: f64,
    lookahead_hours: i64,
) -> BacktestResult {
    run_backtest_with_costs(
        candles,
        weights,
        score_threshold,
        lookahead_hours,
        BacktestCosts::default(),
    )
}

pub fn run_backtest_with_costs(
    candles: &[Candle],
    weights: &score_engine::ScoreWeights,
    score_threshold: f64,
    lookahead_hours: i64,
    costs: BacktestCosts,
) -> BacktestResult {
    let symbol = candles
        .first()
        .map(|candle| candle.symbol.clone())
        .unwrap_or_default();
    let interval = candles
        .first()
        .map(|candle| candle.interval.clone())
        .unwrap_or_default();
    let threshold = score_threshold.clamp(50.0, 100.0);
    let mut signals = Vec::new();
    let mut previous_side = None;
    let mut no_trade_points = 0;

    if candles.len() < SCORE_WINDOW + 1 {
        return result_from_signals(
            ResultMeta {
                symbol,
                interval,
                scope: "candle-only",
                score_threshold: threshold,
                lookahead_hours,
                costs,
                no_trade_points: 0,
            },
            signals,
        );
    }

    for index in (SCORE_WINDOW - 1)..candles.len() - 1 {
        let window = &candles[index + 1 - SCORE_WINDOW..=index];
        let current = &candles[index];
        let input = score_engine::ScoreInput {
            candles: window.to_vec(),
            order_book: OrderBookSnapshot {
                symbol: current.symbol.clone(),
                timestamp: 0,
                bids: Vec::new(),
                asks: Vec::new(),
            },
            funding_rate: FundingRate {
                symbol: current.symbol.clone(),
                timestamp: 0,
                rate: 0.0,
            },
        };
        let score = score_engine::calculate(&input, weights);
        let side = side_from_score(score.value, threshold);
        if side.is_none() {
            no_trade_points += 1;
        }
        if side != previous_side {
            if let Some(current_side) = side {
                if let Some(signal) = record_signal(
                    candles,
                    index,
                    score.value,
                    current_side,
                    lookahead_hours,
                    costs,
                ) {
                    signals.push(signal);
                }
            }
        }
        previous_side = side;
    }

    result_from_signals(
        ResultMeta {
            symbol,
            interval,
            scope: "candle-only",
            score_threshold: threshold,
            lookahead_hours,
            costs,
            no_trade_points,
        },
        signals,
    )
}

pub fn run_full_backtest(
    frames: &[HistoricalMarketFrame],
    weights: &score_engine::ScoreWeights,
    score_threshold: f64,
    lookahead_hours: i64,
    costs: BacktestCosts,
) -> BacktestResult {
    let candles: Vec<Candle> = frames.iter().map(|frame| frame.candle.clone()).collect();
    let symbol = candles
        .first()
        .map(|candle| candle.symbol.clone())
        .unwrap_or_default();
    let interval = candles
        .first()
        .map(|candle| candle.interval.clone())
        .unwrap_or_default();
    let threshold = score_threshold.clamp(50.0, 100.0);
    let mut signals = Vec::new();
    let mut previous_side = None;
    let mut no_trade_points = 0;
    let mut oi_history: VecDeque<OpenInterest> = VecDeque::with_capacity(2);

    if frames.len() < SCORE_WINDOW + 1 {
        return result_from_signals(
            ResultMeta {
                symbol,
                interval,
                scope: "full-market-frame",
                score_threshold: threshold,
                lookahead_hours,
                costs,
                no_trade_points: 0,
            },
            signals,
        );
    }

    for index in 0..frames.len() - 1 {
        if let Some(oi) = frames[index].open_interest.clone() {
            oi_history.push_back(oi);
            while oi_history.len() > 2 {
                oi_history.pop_front();
            }
        }
        if index + 1 < SCORE_WINDOW {
            continue;
        }

        let frame = &frames[index];
        let window = &candles[index + 1 - SCORE_WINDOW..=index];
        let book = frame
            .order_book
            .clone()
            .unwrap_or_else(|| OrderBookSnapshot {
                symbol: frame.candle.symbol.clone(),
                timestamp: 0,
                bids: vec![],
                asks: vec![],
            });
        let funding = frame.funding_rate.clone().unwrap_or_else(|| FundingRate {
            symbol: frame.candle.symbol.clone(),
            timestamp: 0,
            rate: 0.0,
        });
        let score = score_engine::calculate(
            &score_engine::ScoreInput {
                candles: window.to_vec(),
                order_book: book.clone(),
                funding_rate: funding.clone(),
            },
            weights,
        );

        let previous_oi = if oi_history.len() >= 2 {
            oi_history.front()
        } else {
            None
        };
        let current_oi = oi_history.back();
        let previous_candle = &window[window.len() - 2];
        let current_candle = &window[window.len() - 1];
        let price_change = if previous_candle.close > 0.0 {
            (current_candle.close / previous_candle.close - 1.0) * 100.0
        } else {
            0.0
        };
        let derivatives =
            score_engine::derivatives(previous_oi, current_oi, &funding, price_change);
        let source_health = [
            score_engine::DataSourceHealth {
                name: "candles".into(),
                age_ms: 0,
                max_age_ms: 120_000,
                present: true,
            },
            score_engine::DataSourceHealth {
                name: "order_book".into(),
                age_ms: frame
                    .order_book
                    .as_ref()
                    .map(|value| frame.candle.close_time.saturating_sub(value.timestamp))
                    .unwrap_or(i64::MAX),
                max_age_ms: 30_000,
                present: frame.order_book.is_some(),
            },
            score_engine::DataSourceHealth {
                name: "funding".into(),
                age_ms: frame
                    .funding_rate
                    .as_ref()
                    .map(|value| frame.candle.close_time.saturating_sub(value.timestamp))
                    .unwrap_or(i64::MAX),
                max_age_ms: 9 * 60 * 60 * 1_000,
                present: frame.funding_rate.is_some(),
            },
            score_engine::DataSourceHealth {
                name: "open_interest".into(),
                age_ms: current_oi
                    .map(|value| frame.candle.close_time.saturating_sub(value.timestamp))
                    .unwrap_or(i64::MAX),
                max_age_ms: 5 * 60 * 1_000,
                present: oi_history.len() >= 2,
            },
        ];
        let quality = score_engine::data_quality(&source_health);
        let liquidity = score_engine::liquidity_order_flow(&book);
        let volatility = score_engine::volatility(window);
        let evidence = [
            score.volume_anomaly - 50.0,
            score.funding_extreme - 50.0,
            score.order_book_imbalance - 50.0,
            score.rsi_divergence - 50.0,
            derivatives.derivatives_score - 50.0,
        ];
        let signal_quality = score_engine::signal_quality(&evidence);
        let volumes: Vec<f64> = window.iter().map(|candle| candle.volume).collect();
        let anomaly = score_engine::anomaly_zscore(
            &volumes[..volumes.len() - 1],
            volumes[volumes.len() - 1],
        )
        .anomaly_score;
        let risk = score_engine::risk(
            volatility.score,
            liquidity.liquidity_risk,
            quality.score,
            signal_quality.conflict,
            anomaly,
        )
        .score
        .max(derivatives.leverage_score);
        let mut decision = score_engine::meta_decision(
            score.value,
            signal_quality.agreement,
            quality.score,
            risk,
        );
        let shock = score_engine::market_shock(window);
        let stress = score_engine::derivatives_stress(Some(&derivatives));
        let conflict = score_engine::signal_conflict(
            decision.decision,
            score.value,
            Some(derivatives.derivatives_score),
            Some(&liquidity),
        );
        if conflict.force_no_trade || shock.score >= 85.0 || stress.score >= 90.0 {
            decision.decision = score_engine::Decision::NoTrade;
        }

        let side = match decision.decision {
            score_engine::Decision::StrongLong | score_engine::Decision::Long
                if score.value >= threshold =>
            {
                Some(BacktestSide::Long)
            }
            score_engine::Decision::StrongShort | score_engine::Decision::Short
                if score.value <= 100.0 - threshold =>
            {
                Some(BacktestSide::Short)
            }
            _ => None,
        };
        if side.is_none() {
            no_trade_points += 1;
        }
        if side != previous_side {
            if let Some(current_side) = side {
                if let Some(signal) = record_signal(
                    &candles,
                    index,
                    score.value,
                    current_side,
                    lookahead_hours,
                    costs,
                ) {
                    signals.push(signal);
                }
            }
        }
        previous_side = side;
    }

    result_from_signals(
        ResultMeta {
            symbol,
            interval,
            scope: "full-market-frame",
            score_threshold: threshold,
            lookahead_hours,
            costs,
            no_trade_points,
        },
        signals,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use score_engine::ScoreWeights;

    fn candle(index: i64, close: f64, volume: f64) -> Candle {
        Candle {
            symbol: "BTCUSDT".into(),
            interval: "1h".into(),
            open_time: index * 3_600_000,
            close_time: index * 3_600_000 + 3_600_000,
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
    fn supports_short_direction_and_costs() {
        let candles = [
            candle(0, 100.0, 100.0),
            candle(1, 100.0, 100.0),
            candle(2, 90.0, 100.0),
        ];
        let signal = record_signal(
            &candles,
            0,
            20.0,
            BacktestSide::Short,
            2,
            BacktestCosts {
                round_trip_fee_pct: 0.1,
                slippage_pct: 0.1,
                funding_pct: 0.0,
            },
        )
        .unwrap();
        assert!(signal.gross_directional_return_pct > 0.0);
        assert!(signal.return_pct < signal.gross_directional_return_pct);
        assert!(signal.win);
    }

    #[test]
    fn candle_only_scope_is_explicit() {
        let candles: Vec<Candle> = (0..60)
            .map(|index| {
                candle(
                    index,
                    100.0 + index as f64,
                    if index == 50 { 100_000.0 } else { 100.0 },
                )
            })
            .collect();
        let result = run_backtest(&candles, &ScoreWeights::default(), 70.0, 4);
        assert_eq!(result.scope, "candle-only");
    }
}
