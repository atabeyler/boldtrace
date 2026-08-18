//! Admin-only account approval. Registration leaves an account in the
//! `pending` state (see auth.rs); nothing here is reachable without a live
//! session belonging to a user whose `is_admin` flag is true.

use crate::auth::{self, error};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::CookieJar;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingUser {
    pub id: String,
    pub user_code: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct PendingRow {
    id: String,
    user_code: String,
    email: String,
    first_name: String,
    last_name: String,
    created_at: time::OffsetDateTime,
}

impl From<PendingRow> for PendingUser {
    fn from(r: PendingRow) -> Self {
        Self {
            id: r.id,
            user_code: r.user_code,
            email: r.email,
            first_name: r.first_name,
            last_name: r.last_name,
            created_at: r
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

/// Confirms the caller has a live session and `is_admin = true`; returns
/// the pool for the caller to keep using, or the error response to return.
async fn require_admin(
    state: &AppState,
    jar: &CookieJar,
) -> Result<PgPool, axum::response::Response> {
    let pool = auth::require_pool(state).map_err(IntoResponse::into_response)?;
    match auth::session_user(&pool, jar).await {
        Ok(Some(user)) if user.is_admin => Ok(pool),
        Ok(Some(_)) => Err(error(StatusCode::FORBIDDEN, "admin_required").into_response()),
        Ok(None) => Err(error(StatusCode::UNAUTHORIZED, "not_authenticated").into_response()),
        Err(err) => {
            tracing::warn!(error=%err, "admin session lookup failed");
            Err(error(StatusCode::INTERNAL_SERVER_ERROR, "session_check_failed").into_response())
        }
    }
}

pub async fn list_pending(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let pool = match require_admin(&state, &jar).await {
        Ok(pool) => pool,
        Err(resp) => return resp,
    };
    let rows = sqlx::query_as::<_, PendingRow>(
        "SELECT id, user_code, email, first_name, last_name, created_at \
         FROM web_users WHERE status = 'pending' ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await;
    match rows {
        Ok(rows) => Json(rows.into_iter().map(PendingUser::from).collect::<Vec<_>>()).into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "failed to list pending accounts");
            error(StatusCode::INTERNAL_SERVER_ERROR, "list_failed").into_response()
        }
    }
}

async fn set_status(
    state: AppState,
    jar: CookieJar,
    user_id: String,
    status: &'static str,
) -> impl IntoResponse {
    let pool = match require_admin(&state, &jar).await {
        Ok(pool) => pool,
        Err(resp) => return resp,
    };
    let result = sqlx::query("UPDATE web_users SET status = $1 WHERE id = $2 AND status = 'pending'")
        .bind(status)
        .bind(&user_id)
        .execute(&pool)
        .await;
    match result {
        Ok(r) if r.rows_affected() == 1 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => error(StatusCode::NOT_FOUND, "pending_account_not_found").into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "failed to update account status");
            error(StatusCode::INTERNAL_SERVER_ERROR, "update_failed").into_response()
        }
    }
}

pub async fn approve(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    set_status(state, jar, user_id, "approved").await
}

pub async fn reject(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    set_status(state, jar, user_id, "rejected").await
}
