use crate::{explain, meta_decision, DerivativesResult, Explanation, MetaDecision, Score};

#[derive(Debug, Clone, PartialEq)]
pub struct IntelligenceSnapshot {
    pub symbol: String,
    pub timestamp: i64,
    pub score: f64,
    pub derivatives_score: Option<f64>,
    pub data_quality: f64,
    pub agreement: f64,
    pub decision: MetaDecision,
    pub explanation: Explanation,
}

pub fn intelligence_snapshot(score: &Score, derivatives: Option<&DerivativesResult>) -> IntelligenceSnapshot {
    let (derivatives_score, risk, data_quality, agreement) = match derivatives {
        Some(d) => (
            Some(d.derivatives_score),
            d.leverage_score,
            if d.oi_change_pct.is_finite() && d.funding_rate.is_finite() { 85.0 } else { 40.0 },
            (100.0 - (score.value - d.derivatives_score).abs()).clamp(0.0, 100.0),
        ),
        None => (None, 35.0, 55.0, 50.0),
    };
    let decision = meta_decision(score.value, agreement, data_quality, risk);
    let mut factors = vec![
        ("composite_score", score.value),
        ("volume_anomaly", score.volume_anomaly),
        ("funding_extreme", score.funding_extreme),
        ("order_book_imbalance", score.order_book_imbalance),
        ("rsi_divergence", score.rsi_divergence),
    ];
    if let Some(value) = derivatives_score { factors.push(("derivatives", value)); }
    let explanation = explain(&decision, &factors);
    IntelligenceSnapshot { symbol: score.symbol.clone(), timestamp: score.timestamp, score: score.value, derivatives_score, data_quality, agreement, decision, explanation }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_without_derivatives_is_conservative() {
        let score = Score { symbol:"BTCUSDT".into(), timestamp:1, value:80.0, volume_anomaly:80.0, funding_extreme:50.0, order_book_imbalance:60.0, rsi_divergence:70.0 };
        let snapshot = intelligence_snapshot(&score, None);
        assert_eq!(snapshot.data_quality, 55.0);
        assert!(!snapshot.explanation.headline.is_empty());
    }
}
