use crate::{explain, meta_decision, DerivativesResult, Explanation, MetaDecision};
use shared::Score;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalComponents {
    pub volume_anomaly: f64,
    pub funding_extreme: f64,
    pub order_book_imbalance: f64,
    pub rsi_divergence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntelligenceContext {
    pub data_quality: f64,
    pub agreement: f64,
    pub risk: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntelligenceSnapshot {
    pub symbol: String,
    pub timestamp: i64,
    pub score: f64,
    pub components: SignalComponents,
    pub derivatives_score: Option<f64>,
    pub data_quality: f64,
    pub agreement: f64,
    pub decision: MetaDecision,
    pub explanation: Explanation,
}

/// Builds a snapshot from explicitly measured live context.  The caller is
/// responsible for deriving data quality, agreement and risk from the live
/// source state so production decisions never rely on placeholder constants.
pub fn intelligence_snapshot_with_context(
    score: &Score,
    derivatives: Option<&DerivativesResult>,
    context: IntelligenceContext,
) -> IntelligenceSnapshot {
    let derivatives_score = derivatives.map(|d| d.derivatives_score);
    let decision = meta_decision(
        score.value,
        context.agreement.clamp(0.0, 100.0),
        context.data_quality.clamp(0.0, 100.0),
        context.risk.clamp(0.0, 100.0),
    );
    let components = SignalComponents {
        volume_anomaly: score.volume_anomaly,
        funding_extreme: score.funding_extreme,
        order_book_imbalance: score.order_book_imbalance,
        rsi_divergence: score.rsi_divergence,
    };
    let mut factors = vec![
        ("composite_score", score.value),
        ("volume_anomaly", score.volume_anomaly),
        ("funding_extreme", score.funding_extreme),
        ("order_book_imbalance", score.order_book_imbalance),
        ("rsi_divergence", score.rsi_divergence),
    ];
    if let Some(value) = derivatives_score {
        factors.push(("derivatives", value));
    }
    let explanation = explain(&decision, &factors);
    IntelligenceSnapshot {
        symbol: score.symbol.clone(),
        timestamp: score.timestamp,
        score: score.value,
        components,
        derivatives_score,
        data_quality: context.data_quality.clamp(0.0, 100.0),
        agreement: context.agreement.clamp(0.0, 100.0),
        decision,
        explanation,
    }
}

/// Backwards-compatible conservative wrapper used by isolated callers and
/// tests that do not own live source-health state.  Production MarketState
/// uses `intelligence_snapshot_with_context` instead.
pub fn intelligence_snapshot(
    score: &Score,
    derivatives: Option<&DerivativesResult>,
) -> IntelligenceSnapshot {
    let context = match derivatives {
        Some(d) => IntelligenceContext {
            data_quality: 85.0,
            agreement: (100.0 - (score.value - d.derivatives_score).abs()).clamp(0.0, 100.0),
            risk: d.leverage_score,
        },
        None => IntelligenceContext {
            data_quality: 55.0,
            agreement: 50.0,
            risk: 35.0,
        },
    };
    intelligence_snapshot_with_context(score, derivatives, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score() -> Score {
        Score {
            symbol: "BTCUSDT".into(),
            timestamp: 1,
            value: 80.0,
            volume_anomaly: 80.0,
            funding_extreme: 50.0,
            order_book_imbalance: 60.0,
            rsi_divergence: 70.0,
        }
    }

    #[test]
    fn snapshot_without_derivatives_is_conservative() {
        let snapshot = intelligence_snapshot(&score(), None);
        assert_eq!(snapshot.data_quality, 55.0);
        assert!(!snapshot.explanation.headline.is_empty());
    }

    #[test]
    fn measured_bad_data_forces_no_trade() {
        let snapshot = intelligence_snapshot_with_context(
            &score(),
            None,
            IntelligenceContext {
                data_quality: 20.0,
                agreement: 90.0,
                risk: 10.0,
            },
        );
        assert_eq!(snapshot.decision.decision, crate::Decision::NoTrade);
    }

    #[test]
    fn snapshot_retains_signal_components() {
        let snapshot = intelligence_snapshot(&score(), None);
        assert_eq!(snapshot.components.volume_anomaly, 80.0);
        assert_eq!(snapshot.components.funding_extreme, 50.0);
        assert_eq!(snapshot.components.order_book_imbalance, 60.0);
        assert_eq!(snapshot.components.rsi_divergence, 70.0);
    }
}
