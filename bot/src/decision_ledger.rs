use score_engine::IntelligenceSnapshot;
use serde_json::json;
use sqlx::PgPool;

#[derive(Clone)]
pub struct DecisionLedger { pool: PgPool }

impl DecisionLedger {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn append(&self, snapshot: &IntelligenceSnapshot) -> Result<(), sqlx::Error> {
        let rationale = if snapshot.explanation.reasons.is_empty() {
            snapshot.explanation.headline.clone()
        } else {
            snapshot.explanation.reasons.join("; ")
        };
        let metadata = json!({
            "signal_quality": snapshot.decision.signal_quality,
            "risk": snapshot.decision.risk,
            "data_quality": snapshot.data_quality,
            "agreement": snapshot.agreement,
            "derivatives_score": snapshot.derivatives_score,
            "warnings": snapshot.explanation.warnings,
            "headline": snapshot.explanation.headline,
            "engine": "boldtrace-intelligence-v2"
        });
        sqlx::query("INSERT INTO decision_ledger (symbol, score, decision, rationale, confidence, decided_at_millis, metadata) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(&snapshot.symbol)
            .bind(snapshot.score)
            .bind(format!("{:?}", snapshot.decision.decision))
            .bind(rationale)
            .bind(snapshot.decision.confidence)
            .bind(snapshot.timestamp)
            .bind(metadata)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
