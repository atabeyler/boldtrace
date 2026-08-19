//! The Alert Center feed, derived from real persisted decisions in
//! `decision_ledger` rather than a separate invented "alerts" table.
//! Severity is a direct function of the decision's own realized confidence
//! value — never a fabricated label.

use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertItem {
    pub id: String,
    pub severity: String,
    pub symbol: String,
    pub decision: String,
    pub confidence: f64,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    symbol: String,
    decision: String,
    confidence: f64,
    decided_at_millis: i64,
}

fn severity(confidence: f64, decision: &str) -> &'static str {
    if decision == "NoTrade" {
        "INFO"
    } else if confidence >= 85.0 {
        "CRITICAL"
    } else if confidence >= 70.0 {
        "HIGH"
    } else {
        "WATCH"
    }
}

fn millis_to_iso(millis: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(millis / 1000)
        .ok()
        .and_then(|t| t.format(&time::format_description::well_known::Rfc3339).ok())
        .unwrap_or_default()
}

pub async fn alerts(State(state): State<AppState>, Query(q): Query<AlertsQuery>) -> Json<Vec<AlertItem>> {
    let Some(pool) = state.pool.clone() else {
        return Json(Vec::new());
    };
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let rows = sqlx::query_as::<_, Row>(
        "SELECT symbol, decision, confidence, decided_at_millis FROM decision_ledger \
         ORDER BY decided_at_millis DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&pool)
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!(error=%err, "alerts query failed");
            Vec::new()
        }
    };

    Json(
        rows.into_iter()
            .map(|r| AlertItem {
                id: format!("{}-{}", r.symbol, r.decided_at_millis),
                severity: severity(r.confidence, &r.decision).into(),
                symbol: r.symbol,
                decision: r.decision,
                confidence: r.confidence,
                created_at: millis_to_iso(r.decided_at_millis),
            })
            .collect(),
    )
}
