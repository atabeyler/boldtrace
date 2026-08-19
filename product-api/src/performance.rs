//! Realized win-rate performance by outcome horizon, read straight from the
//! `decision_outcomes` table the bot writes as trades resolve. Nothing here
//! is invented: a symbol with no realized outcomes yet returns an empty
//! horizon list rather than a fabricated win rate.

use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HorizonPerformance {
    pub horizon: String,
    pub win_rate: f64,
    pub samples: usize,
    pub avg_directional_return_pct: f64,
    pub reliability: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    samples: i64,
    win_rate: Option<f64>,
    avg_directional_return_pct: Option<f64>,
}

/// Below this many realized outcomes a win rate is too noisy to call
/// reliable; above it, it's a genuinely usable sample.
fn reliability_label(samples: i64) -> &'static str {
    if samples < 30 {
        "LOW"
    } else if samples < 150 {
        "MEDIUM"
    } else {
        "HIGH"
    }
}

fn horizon_label(minutes: i32) -> &'static str {
    match minutes {
        15 => "15m",
        60 => "1h",
        240 => "4h",
        _ => "unknown",
    }
}

pub async fn performance(State(state): State<AppState>, Path(symbol): Path<String>) -> Json<Vec<HorizonPerformance>> {
    let Some(pool) = state.pool.clone() else {
        return Json(Vec::new());
    };
    let symbol = symbol.to_uppercase();
    let mut out = Vec::new();
    for horizon_minutes in [15, 60, 240] {
        let row = sqlx::query_as::<_, Row>(
            "SELECT COUNT(*)::BIGINT AS samples, \
             AVG(CASE WHEN correct THEN 1.0 ELSE 0.0 END)::DOUBLE PRECISION AS win_rate, \
             AVG(directional_return_pct)::DOUBLE PRECISION AS avg_directional_return_pct \
             FROM decision_outcomes WHERE symbol = $1 AND horizon_minutes = $2",
        )
        .bind(&symbol)
        .bind(horizon_minutes)
        .fetch_one(&pool)
        .await;
        match row {
            Ok(row) if row.samples > 0 => out.push(HorizonPerformance {
                horizon: horizon_label(horizon_minutes).into(),
                win_rate: (row.win_rate.unwrap_or(0.0) * 100.0),
                samples: row.samples as usize,
                avg_directional_return_pct: row.avg_directional_return_pct.unwrap_or(0.0),
                reliability: reliability_label(row.samples).into(),
            }),
            Ok(_) => {}
            Err(err) => {
                tracing::warn!(error=%err, %symbol, horizon_minutes, "performance query failed");
            }
        }
    }
    Json(out)
}
