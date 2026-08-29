//! In-memory rolling market state and adaptive score recomputation.
//!
//! Candle histories are isolated by interval. The primary composite uses the
//! closed 1m series when available, while additional intervals participate in
//! multi-timeframe agreement/conflict instead of being mixed into one RSI or
//! volume window.

use score_engine::{
    data_quality, risk, signal_quality, DataSourceHealth, IntelligenceContext, MarketRegime,
    ScoreInput, ScoreWeights, SignalReliability,
};
use shared::{Candle, FundingRate, OpenInterest, OrderBookSnapshot, Score};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const CANDLE_HISTORY: usize = 60;
const PRIMARY_INTERVAL: &str = "1m";
const MIN_WARM_CANDLES: usize = 30;
const MIN_TIMEFRAME_CANDLES: usize = 10;
const CANDLE_MAX_AGE_MS: i64 = 120_000;
const ORDER_BOOK_MAX_AGE_MS: i64 = 30_000;
const FUNDING_MAX_AGE_MS: i64 = 9 * 60 * 60 * 1_000;
const OPEN_INTEREST_MAX_AGE_MS: i64 = 5 * 60 * 1_000;

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Debug, Clone, Copy)]
pub struct SpecializedSnapshot {
    pub sweep: score_engine::SweepResult,
    pub shock: score_engine::ShockResult,
    pub derivatives_stress: score_engine::DerivativesStress,
    pub conflict: score_engine::ConflictResult,
    pub regime_transition: score_engine::RegimeTransition,
    pub cross_market: score_engine::CrossMarketResult,
}

struct SymbolState {
    candles: HashMap<String, VecDeque<Candle>>,
    last_candle_received_at: HashMap<String, i64>,
    order_book: Option<OrderBookSnapshot>,
    funding_rate: Option<FundingRate>,
    open_interest: VecDeque<OpenInterest>,
    latest_score: Option<Score>,
    previous_regime: Option<MarketRegime>,
    adaptive_weights: Option<ScoreWeights>,
}

impl SymbolState {
    fn new() -> Self {
        Self {
            candles: HashMap::new(),
            last_candle_received_at: HashMap::new(),
            order_book: None,
            funding_rate: None,
            open_interest: VecDeque::with_capacity(2),
            latest_score: None,
            previous_regime: None,
            adaptive_weights: None,
        }
    }

    fn primary_interval(&self) -> Option<String> {
        if self.candles.get(PRIMARY_INTERVAL).is_some_and(|history| !history.is_empty()) {
            return Some(PRIMARY_INTERVAL.to_string());
        }
        self.candles
            .iter()
            .filter(|(_, history)| !history.is_empty())
            .max_by_key(|(_, history)| history.len())
            .map(|(interval, _)| interval.clone())
    }

    fn primary_candles(&self) -> Option<&VecDeque<Candle>> {
        let interval = self.primary_interval()?;
        self.candles.get(&interval)
    }

    fn primary_candles_vec(&self) -> Option<Vec<Candle>> {
        Some(self.primary_candles()?.iter().cloned().collect())
    }

    fn recompute(&mut self, symbol: &str, base: &ScoreWeights) -> Option<Score> {
        let candles = self.primary_candles_vec()?;
        let order_book = self.order_book.clone().unwrap_or_else(|| OrderBookSnapshot {
            symbol: symbol.into(),
            timestamp: 0,
            bids: vec![],
            asks: vec![],
        });
        let funding_rate = self.funding_rate.clone().unwrap_or_else(|| FundingRate {
            symbol: symbol.into(),
            timestamp: 0,
            rate: 0.0,
        });
        let input = ScoreInput { candles, order_book, funding_rate };
        let weights = self.adaptive_weights.as_ref().unwrap_or(base);
        let score = score_engine::calculate(&input, weights);
        self.latest_score = Some(score.clone());
        Some(score)
    }

    fn derivatives(&self) -> Option<score_engine::DerivativesResult> {
        let funding = self.funding_rate.as_ref()?;
        let current = self.open_interest.back();
        let previous = if self.open_interest.len() >= 2 { self.open_interest.front() } else { None };
        let candles = self.primary_candles()?;
        let price_change = if candles.len() >= 2 {
            let a = &candles[candles.len() - 2];
            let b = candles.back()?;
            if a.close != 0.0 { (b.close / a.close - 1.0) * 100.0 } else { 0.0 }
        } else {
            0.0
        };
        Some(score_engine::derivatives(previous, current, funding, price_change))
    }

    fn timeframe_metrics(&self, base: &ScoreWeights) -> (f64, f64) {
        let order_book = self.order_book.clone().unwrap_or_else(|| OrderBookSnapshot {
            symbol: self.latest_score.as_ref().map(|s| s.symbol.clone()).unwrap_or_default(),
            timestamp: 0,
            bids: vec![],
            asks: vec![],
        });
        let funding_rate = self.funding_rate.clone().unwrap_or_else(|| FundingRate {
            symbol: order_book.symbol.clone(),
            timestamp: 0,
            rate: 0.0,
        });
        let weights = self.adaptive_weights.as_ref().unwrap_or(base);
        let mut owned = Vec::new();
        for (interval, history) in &self.candles {
            if history.len() < MIN_TIMEFRAME_CANDLES {
                continue;
            }
            let candles: Vec<Candle> = history.iter().cloned().collect();
            let score = score_engine::calculate(
                &ScoreInput {
                    candles: candles.clone(),
                    order_book: order_book.clone(),
                    funding_rate: funding_rate.clone(),
                },
                weights,
            );
            owned.push((interval.clone(), candles, score.value));
        }
        if owned.len() < 2 {
            return (50.0, 25.0);
        }
        let refs: Vec<(&str, &[Candle], f64)> = owned
            .iter()
            .map(|(interval, candles, score)| (interval.as_str(), candles.as_slice(), *score))
            .collect();
        let results = score_engine::multi_timeframe(&refs);
        let directions: Vec<i8> = results
            .iter()
            .map(|result| {
                if result.score >= 58.0 {
                    1
                } else if result.score <= 42.0 {
                    -1
                } else {
                    0
                }
            })
            .collect();
        let positive = directions.iter().filter(|direction| **direction > 0).count();
        let negative = directions.iter().filter(|direction| **direction < 0).count();
        let directional = positive + negative;
        let score_agreement = if directional == 0 {
            60.0
        } else {
            positive.max(negative) as f64 / directional as f64 * 100.0
        };
        let mut conflict = 100.0 - score_agreement;

        let regime_direction = |regime: MarketRegime| match regime {
            MarketRegime::StrongBull | MarketRegime::Bull => 1,
            MarketRegime::StrongBear | MarketRegime::Bear => -1,
            _ => 0,
        };
        let regime_dirs: Vec<i8> = results.iter().map(|result| regime_direction(result.regime)).collect();
        let regime_positive = regime_dirs.iter().any(|direction| *direction > 0);
        let regime_negative = regime_dirs.iter().any(|direction| *direction < 0);
        if regime_positive && regime_negative {
            conflict = (conflict + 40.0).clamp(0.0, 100.0);
        }
        ((100.0 - conflict).clamp(0.0, 100.0), conflict)
    }

    fn intelligence_context(
        &self,
        score: &Score,
        derivatives: Option<&score_engine::DerivativesResult>,
        base: &ScoreWeights,
    ) -> IntelligenceContext {
        let now = now_millis();
        let primary_interval = self.primary_interval();
        let primary_history = primary_interval.as_ref().and_then(|interval| self.candles.get(interval));
        let primary_received = primary_interval
            .as_ref()
            .and_then(|interval| self.last_candle_received_at.get(interval))
            .copied();
        let order_book_ts = self.order_book.as_ref().map(|x| x.timestamp);
        let funding_ts = self.funding_rate.as_ref().map(|x| x.timestamp);
        let oi_ts = self.open_interest.back().map(|x| x.timestamp);
        let warm = primary_history.is_some_and(|history| history.len() >= MIN_WARM_CANDLES);

        let health = [
            DataSourceHealth {
                name: "candles".into(),
                age_ms: primary_received.map(|t| now.saturating_sub(t)).unwrap_or(i64::MAX),
                max_age_ms: CANDLE_MAX_AGE_MS,
                present: warm && primary_received.is_some(),
            },
            DataSourceHealth {
                name: "order_book".into(),
                age_ms: order_book_ts.map(|t| now.saturating_sub(t)).unwrap_or(i64::MAX),
                max_age_ms: ORDER_BOOK_MAX_AGE_MS,
                present: self.order_book.is_some(),
            },
            DataSourceHealth {
                name: "funding".into(),
                age_ms: funding_ts.map(|t| now.saturating_sub(t)).unwrap_or(i64::MAX),
                max_age_ms: FUNDING_MAX_AGE_MS,
                present: self.funding_rate.is_some(),
            },
            DataSourceHealth {
                name: "open_interest".into(),
                age_ms: oi_ts.map(|t| now.saturating_sub(t)).unwrap_or(i64::MAX),
                max_age_ms: OPEN_INTEREST_MAX_AGE_MS,
                present: self.open_interest.len() >= 2,
            },
        ];
        let quality = data_quality(&health);
        let candles = self.primary_candles_vec().unwrap_or_default();
        let liquidity = self.order_book.as_ref().map(score_engine::liquidity_order_flow);
        let volatility = score_engine::volatility(&candles);
        let evidence = [
            score.volume_anomaly - 50.0,
            score.funding_extreme - 50.0,
            score.order_book_imbalance - 50.0,
            score.rsi_divergence - 50.0,
            derivatives.map(|d| d.derivatives_score - 50.0).unwrap_or(0.0),
        ];
        let signal = signal_quality(&evidence);
        let (timeframe_agreement, timeframe_conflict) = self.timeframe_metrics(base);
        let agreement = ((signal.agreement + timeframe_agreement) / 2.0).clamp(0.0, 100.0);
        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();
        let anomaly = if volumes.len() >= 2 {
            score_engine::anomaly_zscore(&volumes[..volumes.len() - 1], volumes[volumes.len() - 1]).anomaly_score
        } else {
            0.0
        };
        let liquidity_risk = liquidity.map(|x| x.liquidity_risk).unwrap_or(100.0);
        let leverage_risk = derivatives.map(|d| d.leverage_score).unwrap_or(70.0);
        let combined_risk = risk(
            volatility.score,
            liquidity_risk,
            quality.score,
            timeframe_conflict,
            anomaly,
        )
        .score
        .max(leverage_risk);

        IntelligenceContext {
            data_quality: quality.score,
            agreement,
            risk: combined_risk,
        }
    }
}

pub struct MarketState {
    symbols: Mutex<HashMap<String, SymbolState>>,
    weights: ScoreWeights,
}

impl MarketState {
    pub fn new(weights: ScoreWeights) -> Self {
        Self { symbols: Mutex::new(HashMap::new()), weights }
    }

    pub fn latest_score(&self, symbol: &str) -> Option<Score> {
        self.symbols
            .lock()
            .expect("market state lock poisoned")
            .get(symbol)
            .and_then(|state| state.latest_score.clone())
    }

    pub fn latest_price(&self, symbol: &str) -> Option<f64> {
        self.symbols.lock().ok()?.get(symbol)?.primary_candles()?.back().map(|c| c.close)
    }

    pub fn regime(&self, symbol: &str) -> Option<MarketRegime> {
        let symbols = self.symbols.lock().ok()?;
        let state = symbols.get(symbol)?;
        let candles = state.primary_candles_vec()?;
        Some(score_engine::market_regime(&candles).regime)
    }

    pub fn adaptive_weights(&self, symbol: &str) -> ScoreWeights {
        self.symbols
            .lock()
            .ok()
            .and_then(|symbols| symbols.get(symbol).and_then(|state| state.adaptive_weights))
            .unwrap_or(self.weights)
    }

    pub fn apply_reliability(&self, symbol: &str, reliability: SignalReliability) -> ScoreWeights {
        let weights = self.weights.adaptive(reliability);
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols.entry(symbol.to_string()).or_insert_with(SymbolState::new);
        state.adaptive_weights = Some(weights);
        if state.primary_candles().is_some() {
            let _ = state.recompute(symbol, &self.weights);
        }
        weights
    }

    pub fn ingest_candle(&self, candle: Candle) -> Option<Score> {
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols.entry(candle.symbol.clone()).or_insert_with(SymbolState::new);
        let interval = candle.interval.clone();
        state.last_candle_received_at.insert(interval.clone(), now_millis());
        let history = state
            .candles
            .entry(interval)
            .or_insert_with(|| VecDeque::with_capacity(CANDLE_HISTORY));
        if let Some(last) = history.back_mut() {
            if last.open_time == candle.open_time {
                *last = candle.clone();
                return state.recompute(&candle.symbol, &self.weights);
            }
        }
        history.push_back(candle.clone());
        if history.len() > CANDLE_HISTORY {
            history.pop_front();
        }
        state.recompute(&candle.symbol, &self.weights)
    }

    pub fn ingest_order_book(&self, snapshot: OrderBookSnapshot) -> Option<Score> {
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols.entry(snapshot.symbol.clone()).or_insert_with(SymbolState::new);
        let symbol = snapshot.symbol.clone();
        state.order_book = Some(snapshot);
        state.recompute(&symbol, &self.weights)
    }

    pub fn ingest_funding_rate(&self, rate: FundingRate) -> Option<Score> {
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols.entry(rate.symbol.clone()).or_insert_with(SymbolState::new);
        let symbol = rate.symbol.clone();
        state.funding_rate = Some(rate);
        state.recompute(&symbol, &self.weights)
    }

    pub fn ingest_open_interest(&self, oi: OpenInterest) -> Option<Score> {
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols.entry(oi.symbol.clone()).or_insert_with(SymbolState::new);
        let symbol = oi.symbol.clone();
        state.open_interest.push_back(oi);
        while state.open_interest.len() > 2 {
            state.open_interest.pop_front();
        }
        state.recompute(&symbol, &self.weights)
    }

    pub fn latest_derivatives(&self, symbol: &str) -> Option<score_engine::DerivativesResult> {
        let symbols = self.symbols.lock().ok()?;
        symbols.get(symbol)?.derivatives()
    }

    pub fn specialized(
        &self,
        symbol: &str,
        decision: score_engine::Decision,
    ) -> Option<SpecializedSnapshot> {
        let mut symbols = self.symbols.lock().ok()?;
        let reference = if symbol != "BTCUSDT" {
            symbols.get("BTCUSDT").and_then(primary_returns)
        } else {
            symbols.get("ETHUSDT").and_then(primary_returns)
        };
        let state = symbols.get_mut(symbol)?;
        let candles = state.primary_candles_vec()?;
        let sweep = score_engine::liquidity_sweep(&candles);
        let shock = score_engine::market_shock(&candles);
        let derivatives = state.derivatives();
        let derivatives_stress = score_engine::derivatives_stress(derivatives.as_ref());
        let liquidity = state.order_book.as_ref().map(score_engine::liquidity_order_flow);
        let composite = state.latest_score.as_ref()?.value;
        let conflict = score_engine::signal_conflict(
            decision,
            composite,
            derivatives.as_ref().map(|d| d.derivatives_score),
            liquidity.as_ref(),
        );
        let current = score_engine::market_regime(&candles).regime;
        let regime_transition = score_engine::regime_transition(state.previous_regime, current, shock.score);
        state.previous_regime = Some(current);
        let primary = returns(&candles);
        let cross_market = reference.as_ref().map_or(
            score_engine::CrossMarketResult { score: 0.0, divergence: 0.0 },
            |reference_returns| score_engine::cross_market(&primary, reference_returns),
        );
        Some(SpecializedSnapshot {
            sweep,
            shock,
            derivatives_stress,
            conflict,
            regime_transition,
            cross_market,
        })
    }

    pub fn intelligence(&self, symbol: &str) -> Option<score_engine::IntelligenceSnapshot> {
        let symbols = self.symbols.lock().ok()?;
        let state = symbols.get(symbol)?;
        let score = state.latest_score.as_ref()?;
        let derivatives = state.derivatives();
        let context = state.intelligence_context(score, derivatives.as_ref(), &self.weights);
        Some(score_engine::intelligence_snapshot_with_context(
            score,
            derivatives.as_ref(),
            context,
        ))
    }
}

fn returns(candles: &[Candle]) -> Vec<f64> {
    candles
        .windows(2)
        .filter_map(|window| {
            (window[0].close > 0.0)
                .then_some((window[1].close / window[0].close - 1.0) * 100.0)
        })
        .collect()
}

fn primary_returns(state: &SymbolState) -> Option<Vec<f64>> {
    let candles = state.primary_candles_vec()?;
    Some(returns(&candles))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(symbol: &str, interval: &str, t: i64) -> Candle {
        Candle {
            symbol: symbol.into(),
            interval: interval.into(),
            open_time: t,
            close_time: t + 60_000,
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: 100.0,
        }
    }

    #[test]
    fn score_can_warm_before_decision_is_trusted() {
        let state = MarketState::new(ScoreWeights::default());
        state.ingest_candle(candle("BTCUSDT", "1m", now_millis() - 60_000));
        assert!(state.latest_score("BTCUSDT").is_some());
        let snapshot = state.intelligence("BTCUSDT").unwrap();
        assert_eq!(snapshot.decision.decision, score_engine::Decision::NoTrade);
        assert_eq!(snapshot.data_quality, 0.0);
    }

    #[test]
    fn timeframe_histories_do_not_mix() {
        let state = MarketState::new(ScoreWeights::default());
        state.ingest_candle(candle("BTCUSDT", "1m", 0));
        state.ingest_candle(candle("BTCUSDT", "5m", 0));
        let symbols = state.symbols.lock().unwrap();
        let symbol = symbols.get("BTCUSDT").unwrap();
        assert_eq!(symbol.candles.get("1m").unwrap().len(), 1);
        assert_eq!(symbol.candles.get("5m").unwrap().len(), 1);
    }

    #[test]
    fn reliability_changes_mature_weights() {
        let state = MarketState::new(ScoreWeights::default());
        let weights = state.apply_reliability(
            "BTCUSDT",
            SignalReliability {
                volume: 90.0,
                funding: 20.0,
                order_book: 30.0,
                rsi: 40.0,
                samples: 200,
            },
        );
        assert!(weights.volume_anomaly > weights.funding_extreme);
    }

    #[test]
    fn keeps_two_oi_points() {
        let state = MarketState::new(ScoreWeights::default());
        for i in 0..3 {
            state.ingest_open_interest(OpenInterest {
                symbol: "BTCUSDT".into(),
                timestamp: i,
                value: 100.0 + i as f64,
            });
        }
        let symbols = state.symbols.lock().unwrap();
        assert_eq!(symbols.get("BTCUSDT").unwrap().open_interest.len(), 2);
    }
}
