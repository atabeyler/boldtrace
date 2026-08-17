//! Volume anomaly signal: how far the latest candle's volume deviates from
//! the recent average, expressed as a 0-100 score.

use shared::Candle;

/// Number of preceding candles used as the baseline for the z-score.
const BASELINE_WINDOW: usize = 24;

/// Absolute z-score at or above which the signal saturates at 100.
const SATURATION_Z_SCORE: f64 = 3.0;

/// Scores how anomalous the most recent candle's volume is relative to the
/// average of up to the preceding [`BASELINE_WINDOW`] candles. `candles`
/// must be sorted oldest-first; the last entry is treated as "current".
/// Returns `0.0` when there is not enough history to establish a baseline.
pub fn volume_anomaly(candles: &[Candle]) -> f64 {
    let Some((latest, history)) = candles.split_last() else {
        return 0.0;
    };
    if history.is_empty() {
        return 0.0;
    }

    let baseline_start = history.len().saturating_sub(BASELINE_WINDOW);
    let baseline = &history[baseline_start..];

    let mean = baseline.iter().map(|c| c.volume).sum::<f64>() / baseline.len() as f64;
    let variance = baseline
        .iter()
        .map(|c| (c.volume - mean).powi(2))
        .sum::<f64>()
        / baseline.len() as f64;
    let std_dev = variance.sqrt();

    if std_dev == 0.0 {
        return if latest.volume == mean { 0.0 } else { 100.0 };
    }

    let z_score = ((latest.volume - mean) / std_dev).abs();
    (z_score / SATURATION_Z_SCORE * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle_with_volume(volume: f64) -> Candle {
        Candle {
            symbol: "BTCUSDT".to_string(),
            interval: "1m".to_string(),
            open_time: 0,
            close_time: 0,
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume,
        }
    }

    #[test]
    fn returns_zero_with_insufficient_history() {
        assert_eq!(volume_anomaly(&[]), 0.0);
        assert_eq!(volume_anomaly(&[candle_with_volume(100.0)]), 0.0);
    }

    #[test]
    fn returns_zero_when_volume_matches_flat_baseline() {
        let candles: Vec<Candle> = (0..10).map(|_| candle_with_volume(100.0)).collect();
        assert_eq!(volume_anomaly(&candles), 0.0);
    }

    #[test]
    fn saturates_on_extreme_spike() {
        let mut candles: Vec<Candle> = (0..10).map(|_| candle_with_volume(100.0)).collect();
        candles.push(candle_with_volume(100_000.0));
        assert_eq!(volume_anomaly(&candles), 100.0);
    }

    #[test]
    fn scores_moderate_spike_between_bounds() {
        let mut candles: Vec<Candle> = vec![
            candle_with_volume(90.0),
            candle_with_volume(100.0),
            candle_with_volume(110.0),
            candle_with_volume(95.0),
            candle_with_volume(105.0),
        ];
        candles.push(candle_with_volume(115.0));
        let score = volume_anomaly(&candles);
        assert!(score > 0.0 && score < 100.0);
    }
}
