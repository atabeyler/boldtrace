//! In-memory market state: keeps a rolling window of recent market data
//! per symbol (fed by exchange-client's Redis publications) and recomputes
//! the composite score via `score-engine` whenever it changes.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use score_engine::{ScoreInput, ScoreWeights};
use shared::{Candle, FundingRate, OrderBookSnapshot, Score};

/// Number of trailing candles kept per symbol; must cover score-engine's
/// signal lookback windows.
const CANDLE_HISTORY: usize = 60;

struct SymbolState {
    candles: VecDeque<Candle>,
    order_book: Option<OrderBookSnapshot>,
    funding_rate: Option<FundingRate>,
    latest_score: Option<Score>,
}

impl SymbolState {
    fn new() -> Self {
        Self {
            candles: VecDeque::with_capacity(CANDLE_HISTORY),
            order_book: None,
            funding_rate: None,
            latest_score: None,
        }
    }

    fn recompute(&mut self, symbol: &str, weights: &ScoreWeights) -> Option<Score> {
        if self.candles.is_empty() {
            return None;
        }
        let order_book = self.order_book.clone().unwrap_or_else(|| OrderBookSnapshot {
            symbol: symbol.to_string(),
            timestamp: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        });
        let funding_rate = self.funding_rate.clone().unwrap_or_else(|| FundingRate {
            symbol: symbol.to_string(),
            timestamp: 0,
            rate: 0.0,
        });
        let input = ScoreInput {
            candles: self.candles.iter().cloned().collect(),
            order_book,
            funding_rate,
        };
        let score = score_engine::calculate(&input, weights);
        self.latest_score = Some(score.clone());
        Some(score)
    }
}

/// Thread-safe, per-symbol market state shared between the Redis
/// subscriber task and the `/tara` and `/alarm` command handlers.
pub struct MarketState {
    symbols: Mutex<HashMap<String, SymbolState>>,
    weights: ScoreWeights,
}

impl MarketState {
    pub fn new(weights: ScoreWeights) -> Self {
        Self {
            symbols: Mutex::new(HashMap::new()),
            weights,
        }
    }

    /// Returns the most recently computed score for `symbol`, if any data
    /// has been received for it yet.
    pub fn latest_score(&self, symbol: &str) -> Option<Score> {
        self.symbols
            .lock()
            .expect("market state lock poisoned")
            .get(symbol)
            .and_then(|state| state.latest_score.clone())
    }

    /// Records a new candle for its symbol/interval and recomputes the
    /// score, replacing same-interval duplicates (e.g. an unclosed candle
    /// being updated tick by tick) rather than appending them.
    pub fn ingest_candle(&self, candle: Candle) -> Option<Score> {
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols.entry(candle.symbol.clone()).or_insert_with(SymbolState::new);

        if let Some(last) = state.candles.back_mut() {
            if last.interval == candle.interval && last.open_time == candle.open_time {
                *last = candle.clone();
                return state.recompute(&candle.symbol, &self.weights);
            }
        }
        state.candles.push_back(candle.clone());
        if state.candles.len() > CANDLE_HISTORY {
            state.candles.pop_front();
        }
        state.recompute(&candle.symbol, &self.weights)
    }

    pub fn ingest_order_book(&self, snapshot: OrderBookSnapshot) -> Option<Score> {
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols
            .entry(snapshot.symbol.clone())
            .or_insert_with(SymbolState::new);
        let symbol = snapshot.symbol.clone();
        state.order_book = Some(snapshot);
        state.recompute(&symbol, &self.weights)
    }

    pub fn ingest_funding_rate(&self, funding_rate: FundingRate) -> Option<Score> {
        let mut symbols = self.symbols.lock().expect("market state lock poisoned");
        let state = symbols
            .entry(funding_rate.symbol.clone())
            .or_insert_with(SymbolState::new);
        let symbol = funding_rate.symbol.clone();
        state.funding_rate = Some(funding_rate);
        state.recompute(&symbol, &self.weights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(symbol: &str, open_time: i64, volume: f64) -> Candle {
        Candle {
            symbol: symbol.to_string(),
            interval: "1m".to_string(),
            open_time,
            close_time: open_time + 60_000,
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume,
        }
    }

    #[test]
    fn no_score_before_any_data() {
        let state = MarketState::new(ScoreWeights::default());
        assert!(state.latest_score("BTCUSDT").is_none());
    }

    #[test]
    fn scores_after_first_candle() {
        let state = MarketState::new(ScoreWeights::default());
        state.ingest_candle(candle("BTCUSDT", 0, 100.0));
        assert!(state.latest_score("BTCUSDT").is_some());
    }

    #[test]
    fn same_open_time_updates_in_place_instead_of_appending() {
        let state = MarketState::new(ScoreWeights::default());
        state.ingest_candle(candle("BTCUSDT", 0, 100.0));
        state.ingest_candle(candle("BTCUSDT", 0, 150.0));
        let symbols = state.symbols.lock().unwrap();
        assert_eq!(symbols.get("BTCUSDT").unwrap().candles.len(), 1);
    }

    #[test]
    fn caps_candle_history() {
        let state = MarketState::new(ScoreWeights::default());
        for i in 0..(CANDLE_HISTORY + 10) {
            state.ingest_candle(candle("BTCUSDT", i as i64 * 60_000, 100.0));
        }
        let symbols = state.symbols.lock().unwrap();
        assert_eq!(symbols.get("BTCUSDT").unwrap().candles.len(), CANDLE_HISTORY);
    }
}
