//! RSI + price divergence signal: how much the RSI's trend disagrees with
//! price's trend over a recent window, expressed as a 0-100 score.

use shared::Candle;

const RSI_PERIOD: usize = 14;
const DIVERGENCE_LOOKBACK: usize = 5;

/// Wilder's RSI over `closes`, using the standard smoothing method.
/// Returns `None` when there aren't at least `period + 1` closes.
fn wilder_rsi(closes: &[f64], period: usize) -> Option<f64> {
    if closes.len() < period + 1 {
        return None;
    }

    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for window in closes[..=period].windows(2) {
        let change = window[1] - window[0];
        if change > 0.0 {
            avg_gain += change;
        } else {
            avg_loss += -change;
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    for window in closes[period..].windows(2) {
        let change = window[1] - window[0];
        let (gain, loss) = if change > 0.0 { (change, 0.0) } else { (0.0, -change) };
        avg_gain = (avg_gain * (period - 1) as f64 + gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + loss) / period as f64;
    }

    if avg_loss == 0.0 {
        return Some(100.0);
    }
    let rs = avg_gain / avg_loss;
    Some(100.0 - (100.0 / (1.0 + rs)))
}

/// Scores the disagreement between price direction and RSI direction over
/// the last [`DIVERGENCE_LOOKBACK`] candles: price making a new extreme
/// while RSI moves the opposite way is a classic reversal warning signal.
/// `candles` must be sorted oldest-first. Returns `0.0` when there isn't
/// enough history, or when price and RSI agree (no divergence).
pub fn rsi_divergence(candles: &[Candle]) -> f64 {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    if closes.len() < RSI_PERIOD + DIVERGENCE_LOOKBACK + 1 {
        return 0.0;
    }

    let earlier_closes = &closes[..closes.len() - DIVERGENCE_LOOKBACK];
    let (Some(rsi_now), Some(rsi_before)) = (
        wilder_rsi(&closes, RSI_PERIOD),
        wilder_rsi(earlier_closes, RSI_PERIOD),
    ) else {
        return 0.0;
    };

    let price_now = *closes.last().expect("checked non-empty above");
    let price_before = closes[closes.len() - 1 - DIVERGENCE_LOOKBACK];

    let price_change = price_now - price_before;
    let rsi_change = rsi_now - rsi_before;

    let diverges = (price_change > 0.0 && rsi_change < 0.0) || (price_change < 0.0 && rsi_change > 0.0);
    if diverges {
        rsi_change.abs().clamp(0.0, 100.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle_with_close(close: f64) -> Candle {
        Candle {
            symbol: "BTCUSDT".to_string(),
            interval: "1m".to_string(),
            open_time: 0,
            close_time: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        }
    }

    #[test]
    fn returns_zero_with_insufficient_history() {
        let candles: Vec<Candle> = (0..10).map(|i| candle_with_close(i as f64)).collect();
        assert_eq!(rsi_divergence(&candles), 0.0);
    }

    #[test]
    fn returns_zero_when_price_and_rsi_agree() {
        // Steadily rising closes: price and RSI both trend up, no divergence.
        let candles: Vec<Candle> = (0..30).map(|i| candle_with_close(100.0 + i as f64)).collect();
        assert_eq!(rsi_divergence(&candles), 0.0);
    }

    #[test]
    fn detects_bearish_divergence() {
        // Rises steadily, then makes one final higher high on a losing streak
        // in RSI terms (small down-moves right before it), which should
        // register as a divergence.
        let mut closes: Vec<f64> = (0..24).map(|i| 100.0 + i as f64).collect();
        closes.extend([122.5, 122.0, 121.5, 121.0, 125.0]);
        let candles: Vec<Candle> = closes.into_iter().map(candle_with_close).collect();
        let score = rsi_divergence(&candles);
        assert!(score > 0.0);
    }
}
