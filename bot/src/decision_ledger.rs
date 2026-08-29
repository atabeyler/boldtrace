use crate::outcome_engine::Outcome;
use score_engine::{IntelligenceSnapshot, SignalReliability};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct LedgerRecord {
    pub symbol: String,
    pub score: f64,
    pub decision: String,
    pub rationale: String,
    pub confidence: f64,
    pub decided_at_millis: i64,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionPerformance {
    pub samples: usize,
    pub avg_score: f64,
    pub avg_confidence: f64,
    pub avg_risk: f64,
    pub avg_data_quality: f64,
    pub avg_agreement: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomePerformance {
    pub samples: usize,
    pub win_rate: f64,
    pub avg_directional_return_pct: f64,
}

#[derive(Clone)]
pub struct DecisionLedger {
    pool: PgPool,
}

impl DecisionLedger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn append(&self, snapshot: &IntelligenceSnapshot) -> Result<(), sqlx::Error> {
        let rationale = if snapshot.explanation.reasons.is_empty() {
            snapshot.explanation.headline.clone()
        } else {
            snapshot.explanation.reasons.join("; ")
        };
        let metadata = json!({
            "volume_anomaly": snapshot.components.volume_anomaly,
            "funding_extreme": snapshot.components.funding_extreme,
            "order_book_imbalance": snapshot.components.order_book_imbalance,
            "rsi_divergence": snapshot.components.rsi_divergence,
            "signal_quality": snapshot.decision.signal_quality,
            "risk": snapshot.decision.risk,
            "data_quality": snapshot.data_quality,
            "agreement": snapshot.agreement,
            "derivatives_score": snapshot.derivatives_score,
            "warnings": snapshot.explanation.warnings,
            "headline": snapshot.explanation.headline,
            "engine": "boldtrace-intelligence-v3"
        });
        sqlx::query(
            "INSERT INTO decision_ledger \
             (symbol,score,decision,rationale,confidence,decided_at_millis,metadata) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
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

    pub async fn append_outcome(&self, outcome: &Outcome) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO decision_outcomes \
             (symbol,decision,decided_at_millis,horizon_minutes,entry_price,exit_price,return_pct,directional_return_pct,correct,evaluated_at_millis) \
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
             ON CONFLICT(symbol,decided_at_millis,horizon_minutes) DO NOTHING",
        )
        .bind(&outcome.symbol)
        .bind(&outcome.decision)
        .bind(outcome.decided_at_millis)
        .bind(outcome.horizon_minutes)
        .bind(outcome.entry_price)
        .bind(outcome.exit_price)
        .bind(outcome.return_pct)
        .bind(outcome.directional_return_pct)
        .bind(outcome.correct)
        .bind(outcome.evaluated_at_millis)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Symbols that already have persisted 60-minute realized outcomes.
    /// Used at startup to restore adaptive weights before new live decisions
    /// are allowed to learn from an empty runtime cache.
    pub async fn reliability_symbols(&self) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT symbol FROM decision_outcomes WHERE horizon_minutes = 60 ORDER BY symbol",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn signal_reliability(
        &self,
        symbol: &str,
        limit: i64,
    ) -> Result<SignalReliability, sqlx::Error> {
        let rows: Vec<(bool, Value)> = sqlx::query_as(
            "SELECT o.correct,l.metadata \
             FROM decision_outcomes o \
             JOIN decision_ledger l \
               ON l.symbol=o.symbol AND l.decided_at_millis=o.decided_at_millis \
             WHERE o.symbol=$1 AND o.horizon_minutes=60 \
             ORDER BY o.evaluated_at_millis DESC LIMIT $2",
        )
        .bind(symbol)
        .bind(limit.clamp(1, 2000))
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(SignalReliability::neutral());
        }
        let mut hit = [0.0; 4];
        let mut total = [0.0; 4];
        for (correct, metadata) in &rows {
            for (i, key) in [
                "volume_anomaly",
                "funding_extreme",
                "order_book_imbalance",
                "rsi_divergence",
            ]
            .iter()
            .enumerate()
            {
                let strength = metadata.get(*key).and_then(Value::as_f64).unwrap_or(50.0);
                let conviction = (strength - 50.0).abs() / 50.0;
                if conviction >= 0.20 {
                    total[i] += conviction;
                    if *correct {
                        hit[i] += conviction;
                    }
                }
            }
        }
        let reliability = |i: usize| {
            if total[i] > 0.0 {
                (hit[i] / total[i] * 100.0).clamp(0.0, 100.0)
            } else {
                50.0
            }
        };
        Ok(SignalReliability {
            volume: reliability(0),
            funding: reliability(1),
            order_book: reliability(2),
            rsi: reliability(3),
            samples: rows.len(),
        })
    }

    pub async fn outcome_performance(
        &self,
        symbol: &str,
        horizon_minutes: i32,
        limit: i64,
    ) -> Result<Option<OutcomePerformance>, sqlx::Error> {
        let rows: (i64, Option<f64>, Option<f64>) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT, \
                    AVG(CASE WHEN correct THEN 1.0 ELSE 0.0 END)::DOUBLE PRECISION, \
                    AVG(directional_return_pct)::DOUBLE PRECISION \
             FROM ( \
               SELECT correct,directional_return_pct \
               FROM decision_outcomes \
               WHERE symbol=$1 AND horizon_minutes=$2 \
               ORDER BY evaluated_at_millis DESC LIMIT $3 \
             ) x",
        )
        .bind(symbol)
        .bind(horizon_minutes)
        .bind(limit.clamp(1, 5000))
        .fetch_one(&self.pool)
        .await?;
        if rows.0 == 0 {
            return Ok(None);
        }
        Ok(Some(OutcomePerformance {
            samples: rows.0 as usize,
            win_rate: rows.1.unwrap_or(0.0) * 100.0,
            avg_directional_return_pct: rows.2.unwrap_or(0.0),
        }))
    }

    pub async fn replay(&self, symbol: &str, limit: i64) -> Result<Vec<LedgerRecord>, sqlx::Error> {
        sqlx::query_as::<_, LedgerRecord>(
            "SELECT symbol,score,decision,rationale,confidence,decided_at_millis,metadata \
             FROM decision_ledger WHERE symbol=$1 \
             ORDER BY decided_at_millis DESC LIMIT $2",
        )
        .bind(symbol)
        .bind(limit.clamp(1, 1000))
        .fetch_all(&self.pool)
        .await
    }

    pub async fn performance(
        &self,
        symbol: &str,
        limit: i64,
    ) -> Result<Option<DecisionPerformance>, sqlx::Error> {
        let rows = self.replay(symbol, limit).await?;
        if rows.is_empty() {
            return Ok(None);
        }
        let n = rows.len() as f64;
        let mut score = 0.0;
        let mut confidence = 0.0;
        let mut risk = 0.0;
        let mut quality = 0.0;
        let mut agreement = 0.0;
        for row in &rows {
            score += row.score;
            confidence += row.confidence;
            risk += row.metadata.get("risk").and_then(Value::as_f64).unwrap_or(0.0);
            quality += row
                .metadata
                .get("data_quality")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            agreement += row
                .metadata
                .get("agreement")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
        }
        Ok(Some(DecisionPerformance {
            samples: rows.len(),
            avg_score: score / n,
            avg_confidence: confidence / n,
            avg_risk: risk / n,
            avg_data_quality: quality / n,
            avg_agreement: agreement / n,
        }))
    }
}
