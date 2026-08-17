//! Funding rate extremity signal: how far the current funding rate sits
//! from neutral, expressed as a 0-100 score.

use shared::FundingRate;

/// Absolute funding rate (as a fraction, e.g. `0.001` = 0.1%) at or beyond
/// which the signal saturates at 100. Typical Binance perpetual funding
/// rates sit within a few hundredths of a percent under normal conditions;
/// moves beyond this threshold are considered extreme.
const SATURATION_RATE: f64 = 0.001;

/// Scores how extreme `funding_rate` is relative to neutral (`0.0`).
pub fn funding_extreme(funding_rate: &FundingRate) -> f64 {
    (funding_rate.rate.abs() / SATURATION_RATE * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn funding_rate(rate: f64) -> FundingRate {
        FundingRate {
            symbol: "BTCUSDT".to_string(),
            timestamp: 0,
            rate,
        }
    }

    #[test]
    fn neutral_rate_scores_zero() {
        assert_eq!(funding_extreme(&funding_rate(0.0)), 0.0);
    }

    #[test]
    fn saturates_beyond_threshold_in_either_direction() {
        assert_eq!(funding_extreme(&funding_rate(0.01)), 100.0);
        assert_eq!(funding_extreme(&funding_rate(-0.01)), 100.0);
    }

    #[test]
    fn scores_partial_extremity_proportionally() {
        assert_eq!(funding_extreme(&funding_rate(0.0005)), 50.0);
    }
}
