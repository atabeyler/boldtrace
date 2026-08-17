//! Configurable weights for combining individual signals into the
//! composite score. Never hardcoded at the call site — read from
//! environment or constructed explicitly by the caller (e.g. `backtest`).

/// Relative weight of each signal in the composite score. Weights are
/// normalized (made to sum to `1.0`) before being applied, so callers can
/// supply any positive values.
#[derive(Debug, Clone, Copy)]
pub struct ScoreWeights {
    pub volume_anomaly: f64,
    pub funding_extreme: f64,
    pub order_book_imbalance: f64,
    pub rsi_divergence: f64,
}

impl ScoreWeights {
    /// Reads weights from environment variables, falling back to
    /// `Default::default()` for any that are unset or unparsable.
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            volume_anomaly: env_weight("SCORE_WEIGHT_VOLUME_ANOMALY", defaults.volume_anomaly),
            funding_extreme: env_weight("SCORE_WEIGHT_FUNDING_EXTREME", defaults.funding_extreme),
            order_book_imbalance: env_weight(
                "SCORE_WEIGHT_ORDER_BOOK_IMBALANCE",
                defaults.order_book_imbalance,
            ),
            rsi_divergence: env_weight("SCORE_WEIGHT_RSI_DIVERGENCE", defaults.rsi_divergence),
        }
        .normalized()
    }

    /// Returns weights rescaled so they sum to `1.0`. Falls back to equal
    /// weights if the sum is zero or negative.
    pub fn normalized(self) -> Self {
        let sum = self.volume_anomaly
            + self.funding_extreme
            + self.order_book_imbalance
            + self.rsi_divergence;
        if sum <= 0.0 {
            return Self::default();
        }
        Self {
            volume_anomaly: self.volume_anomaly / sum,
            funding_extreme: self.funding_extreme / sum,
            order_book_imbalance: self.order_book_imbalance / sum,
            rsi_divergence: self.rsi_divergence / sum,
        }
    }
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            volume_anomaly: 0.25,
            funding_extreme: 0.25,
            order_book_imbalance: 0.25,
            rsi_divergence: 0.25,
        }
    }
}

fn env_weight(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_to_sum_one() {
        let weights = ScoreWeights {
            volume_anomaly: 1.0,
            funding_extreme: 1.0,
            order_book_imbalance: 1.0,
            rsi_divergence: 1.0,
        }
        .normalized();
        let sum = weights.volume_anomaly
            + weights.funding_extreme
            + weights.order_book_imbalance
            + weights.rsi_divergence;
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn falls_back_to_default_when_sum_is_zero() {
        let weights = ScoreWeights {
            volume_anomaly: 0.0,
            funding_extreme: 0.0,
            order_book_imbalance: 0.0,
            rsi_divergence: 0.0,
        }
        .normalized();
        assert_eq!(weights.volume_anomaly, ScoreWeights::default().volume_anomaly);
    }
}
