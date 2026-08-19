//! Adaptive-weighting transparency for a symbol: how many realized
//! outcomes have been observed, and how reliable each signal has been
//! historically. The *current* applied weight per engine is read from the
//! live intelligence snapshot (the same value `EngineMatrix` shows) rather
//! than recomputed here, so this never risks drifting from what the
//! running scorer actually used.

use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use score_engine::{MIN_SAMPLES_FOR_ADAPTATION, WEIGHT_MAX, WEIGHT_MIN};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineReliability {
    pub name: String,
    pub reliability: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningState {
    pub samples: usize,
    pub min_sample_gate: usize,
    pub weight_min: f64,
    pub weight_max: f64,
    pub engines: Vec<EngineReliability>,
}

#[derive(sqlx::FromRow)]
struct Row {
    correct: bool,
    metadata: Value,
}

/// Mirrors the bot's own `signal_reliability` query (see
/// `bot/src/decision_ledger.rs`) so the number shown here is the same
/// realized-outcome signal the live scorer's adaptation is driven by, not
/// a separate estimate that could disagree with it.
pub async fn learning(State(state): State<AppState>, Path(symbol): Path<String>) -> Json<LearningState> {
    let empty = LearningState {
        samples: 0,
        min_sample_gate: MIN_SAMPLES_FOR_ADAPTATION,
        weight_min: WEIGHT_MIN,
        weight_max: WEIGHT_MAX,
        engines: Vec::new(),
    };
    let Some(pool) = state.pool.clone() else {
        return Json(empty);
    };
    let symbol = symbol.to_uppercase();
    let rows = sqlx::query_as::<_, Row>(
        "SELECT o.correct, l.metadata FROM decision_outcomes o \
         JOIN decision_ledger l ON l.symbol = o.symbol AND l.decided_at_millis = o.decided_at_millis \
         WHERE o.symbol = $1 AND o.horizon_minutes = 60 ORDER BY o.evaluated_at_millis DESC LIMIT 500",
    )
    .bind(&symbol)
    .fetch_all(&pool)
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!(error=%err, %symbol, "learning reliability query failed");
            return Json(empty);
        }
    };
    if rows.is_empty() {
        return Json(empty);
    }

    let keys = ["volume_anomaly", "funding_extreme", "order_book_imbalance", "rsi_divergence"];
    let labels = ["Volume Anomaly", "Funding Extreme", "Order Book Imbalance", "RSI Divergence"];
    let mut hit = [0.0f64; 4];
    let mut total = [0.0f64; 4];
    for row in &rows {
        for (i, key) in keys.iter().enumerate() {
            let strength = row.metadata.get(*key).and_then(Value::as_f64).unwrap_or(50.0);
            let conviction = (strength - 50.0).abs() / 50.0;
            if conviction >= 0.20 {
                total[i] += conviction;
                if row.correct {
                    hit[i] += conviction;
                }
            }
        }
    }
    let engines = labels
        .iter()
        .enumerate()
        .map(|(i, name)| EngineReliability {
            name: (*name).into(),
            reliability: if total[i] > 0.0 { (hit[i] / total[i] * 100.0).clamp(0.0, 100.0) } else { 50.0 },
        })
        .collect();

    Json(LearningState {
        samples: rows.len(),
        min_sample_gate: MIN_SAMPLES_FOR_ADAPTATION,
        weight_min: WEIGHT_MIN,
        weight_max: WEIGHT_MAX,
        engines,
    })
}
