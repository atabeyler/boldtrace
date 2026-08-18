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
    pub country: String,
    pub national_id: String,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct PendingRow {
    id: String,
    user_code: String,
    email: String,
    first_name: String,
    last_name: String,
    country: String,
    national_id: String,
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
            country: r.country,
            national_id: r.national_id,
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
        "SELECT id, user_code, email, first_name, last_name, country, national_id, created_at \
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
    let result = sqlx::query_as::<_, (String, String)>(
        "UPDATE web_users SET status = $1 WHERE id = $2 AND status = 'pending' RETURNING email, first_name",
    )
    .bind(status)
    .bind(&user_id)
    .fetch_optional(&pool)
    .await;
    match result {
        Ok(Some((email, first_name))) => {
            if status == "approved" {
                crate::email::notify_applicant_approved(&state.email, &email, &first_name).await;
            } else {
                crate::email::notify_applicant_rejected(&state.email, &email, &first_name).await;
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(None) => error(StatusCode::NOT_FOUND, "pending_account_not_found").into_response(),
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationAlert {
    pub id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub expected_country: String,
    pub detected_country: String,
    pub ip: String,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct LocationAlertRow {
    id: String,
    email: String,
    first_name: String,
    last_name: String,
    expected_country: String,
    detected_country: String,
    ip: String,
    created_at: time::OffsetDateTime,
}

impl From<LocationAlertRow> for LocationAlert {
    fn from(r: LocationAlertRow) -> Self {
        Self {
            id: r.id,
            email: r.email,
            first_name: r.first_name,
            last_name: r.last_name,
            expected_country: r.expected_country,
            detected_country: r.detected_country,
            ip: r.ip,
            created_at: r
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

/// Admin-granted window during which a user's login is exempt from the
/// country check, so a traveling user isn't locked out for days.
const LOCATION_OVERRIDE_HOURS: i64 = 24;

pub async fn list_location_alerts(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let pool = match require_admin(&state, &jar).await {
        Ok(pool) => pool,
        Err(resp) => return resp,
    };
    let rows = sqlx::query_as::<_, LocationAlertRow>(
        "SELECT a.id, a.email, u.first_name, u.last_name, a.expected_country, a.detected_country, a.ip, a.created_at \
         FROM login_location_alerts a JOIN web_users u ON u.id = a.user_id \
         WHERE a.resolved = false ORDER BY a.created_at DESC",
    )
    .fetch_all(&pool)
    .await;
    match rows {
        Ok(rows) => Json(rows.into_iter().map(LocationAlert::from).collect::<Vec<_>>()).into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "failed to list location alerts");
            error(StatusCode::INTERNAL_SERVER_ERROR, "list_failed").into_response()
        }
    }
}

/// Marks the alert resolved and grants the affected account a temporary
/// exemption from the country check, so the traveling user can sign back in.
pub async fn allow_location(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(alert_id): Path<String>,
) -> impl IntoResponse {
    let pool = match require_admin(&state, &jar).await {
        Ok(pool) => pool,
        Err(resp) => return resp,
    };
    let resolved = sqlx::query_as::<_, (String,)>(
        "UPDATE login_location_alerts SET resolved = true WHERE id = $1 AND resolved = false RETURNING user_id",
    )
    .bind(&alert_id)
    .fetch_optional(&pool)
    .await;
    let user_id = match resolved {
        Ok(Some((user_id,))) => user_id,
        Ok(None) => return error(StatusCode::NOT_FOUND, "location_alert_not_found").into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "failed to resolve location alert");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "update_failed").into_response();
        }
    };
    let until = time::OffsetDateTime::now_utc() + time::Duration::hours(LOCATION_OVERRIDE_HOURS);
    if let Err(err) = sqlx::query("UPDATE web_users SET location_override_until = $1 WHERE id = $2")
        .bind(until)
        .bind(&user_id)
        .execute(&pool)
        .await
    {
        tracing::warn!(error=%err, "failed to grant location override");
        return error(StatusCode::INTERNAL_SERVER_ERROR, "update_failed").into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}
