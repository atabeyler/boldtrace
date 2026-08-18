//! Pure, stateless composite scoring logic for market data. No I/O: every
//! function here takes data in and returns a value, with no network,
//! filesystem, or database access.

pub mod advanced;
mod input;
mod signals;
mod weights;

pub use advanced::{market_regime, market_structure, volatility, MarketRegime, RegimeResult, StructureResult, StructureState, VolatilityResult, VolatilityState};
pub use input::ScoreInput;
pub use weights::ScoreWeights;

use shared::Score;

/// Computes the composite 0-100 score for `input` using `weights` to
/// combine the four underlying signals: volume anomaly, funding rate
/// extremity, order book imbalance, and RSI/price divergence.
pub fn calculate(input: &ScoreInput, weights: &ScoreWeights) -> Score {
    let weights = weights.normalized();

    let volume_anomaly = signals::volume_anomaly(&input.candles);
    let funding_extreme = signals::funding_extreme(&input.funding_rate);
    let order_book_imbalance = signals::order_book_imbalance(&input.order_book);
    let rsi_divergence = signals::rsi_divergence(&input.candles);

    let value = volume_anomaly * weights.volume_anomaly
        + funding_extreme * weights.funding_extreme
        + order_book_imbalance * weights.order_book_imbalance
        + rsi_divergence * weights.rsi_divergence;

    let (symbol, timestamp) = input
        .candles
        .last()
        .map(|candle| (candle.symbol.clone(), candle.close_time))
        .unwrap_or_else(|| (input.order_book.symbol.clone(), input.order_book.timestamp));

    Score {
        symbol,
        timestamp,
        value: value.clamp(0.0, 100.0),
        volume_anomaly,
        funding_extreme,
        order_book_imbalance,
        rsi_divergence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{Candle, FundingRate, OrderBookLevel, OrderBookSnapshot};

    fn sample_candles() -> Vec<Candle> {
        (0..40)
            .map(|i| Candle {
                symbol: "BTCUSDT".to_string(),
                interval: "1m".to_string(),
                open_time: i * 60_000,
                close_time: i * 60_000 + 60_000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0 + i as f64,
                volume: if i < 39 { 100.0 } else { 500.0 },
            })
            .collect()
    }

    fn sample_input() -> ScoreInput {
        ScoreInput {
            candles: sample_candles(),
            order_book: OrderBookSnapshot {
                symbol: "BTCUSDT".to_string(),
                timestamp: 1_000,
                bids: vec![OrderBookLevel { price: 100.0, quantity: 10.0 }],
                asks: vec![OrderBookLevel { price: 101.0, quantity: 10.0 }],
            },
            funding_rate: FundingRate {
                symbol: "BTCUSDT".to_string(),
                timestamp: 1_000,
                rate: 0.0,
            },
        }
    }

    #[test]
    fn composite_score_is_within_bounds() {
        let score = calculate(&sample_input(), &ScoreWeights::default());
        assert!((0.0..=100.0).contains(&score.value));
        assert_eq!(score.symbol, "BTCUSDT");
    }

    #[test]
    fn weights_change_the_composite_score() {
        let input = sample_input();
        let equal = calculate(&input, &ScoreWeights::default());
        let volume_only = calculate(
            &input,
            &ScoreWeights {
                volume_anomaly: 1.0,
                funding_extreme: 0.0,
                order_book_imbalance: 0.0,
                rsi_divergence: 0.0,
            },
        );
        assert_eq!(volume_only.value, volume_only.volume_anomaly);
        assert_ne!(equal.value, volume_only.value);
    }

    #[test]
    fn falls_back_to_order_book_symbol_without_candles() {
        let mut input = sample_input();
        input.candles.clear();
        let score = calculate(&input, &ScoreWeights::default());
        assert_eq!(score.symbol, "BTCUSDT");
        assert_eq!(score.timestamp, 1_000);
    }
}
