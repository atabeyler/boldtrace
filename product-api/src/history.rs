//! Realized decision history, read from the persisted `decision_ledger` /
//! `decision_outcomes` tables — the same durable audit trail the bot writes
//! to. Nothing here is invented: an unavailable database returns an empty
//! list, never fabricated rows.

use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub symbol: String,
    pub decision: String,
    pub confidence: f64,
    pub horizon: String,
    pub realized_return: Option<f64>,
    pub outcome: String,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    symbol: String,
    decision: String,
    confidence: f64,
    horizon_minutes: i32,
    directional_return_pct: f64,
    correct: bool,
    decided_at_millis: i64,
}

fn horizon_label(minutes: i32) -> &'static str {
    match minutes {
        15 => "15m",
        60 => "1h",
        240 => "4h",
        _ => "unknown",
    }
}

fn millis_to_iso(millis: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(millis / 1000)
        .ok()
        .and_then(|t| t.format(&time::format_description::well_known::Rfc3339).ok())
        .unwrap_or_default()
}

pub async fn history(
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> Json<Vec<HistoryItem>> {
    let Some(pool) = state.pool.clone() else {
        return Json(Vec::new());
    };
    let limit = q.limit.unwrap_or(50).clamp(1, 250);

    const BASE_QUERY: &str = "SELECT l.symbol, l.decision, l.confidence, o.horizon_minutes, \
         o.directional_return_pct, o.correct, o.decided_at_millis \
         FROM decision_outcomes o \
         JOIN decision_ledger l ON l.symbol = o.symbol AND l.decided_at_millis = o.decided_at_millis";

    let rows: Result<Vec<Row>, sqlx::Error> = if let Some(symbol) = q.symbol.as_deref() {
        let query = format!("{BASE_QUERY} WHERE o.symbol = $1 ORDER BY o.evaluated_at_millis DESC LIMIT $2");
        sqlx::query_as(&query)
            .bind(symbol)
            .bind(limit)
            .fetch_all(&pool)
            .await
    } else {
        let query = format!("{BASE_QUERY} ORDER BY o.evaluated_at_millis DESC LIMIT $1");
        sqlx::query_as(&query).bind(limit).fetch_all(&pool).await
    };

    let rows = match rows {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!(error=%err, "history query failed");
            Vec::new()
        }
    };

    Json(
        rows.into_iter()
            .map(|r| HistoryItem {
                id: format!("{}-{}-{}", r.symbol, r.decided_at_millis, r.horizon_minutes),
                symbol: r.symbol,
                decision: r.decision,
                confidence: r.confidence,
                horizon: horizon_label(r.horizon_minutes).into(),
                realized_return: Some(r.directional_return_pct),
                outcome: if r.correct { "WIN".into() } else { "LOSS".into() },
                created_at: millis_to_iso(r.decided_at_millis),
            })
            .collect(),
    )
}
