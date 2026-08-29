use crate::auth;
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::CookieJar;

/// Server-side product boundary: a valid approved session and acceptance of
/// the currently active terms version are required before market intelligence
/// or realized-performance data is exposed.
pub async fn require_product_access(
    State(state): State<AppState>,
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Response {
    let Some(pool) = state.pool.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"database_unavailable"}))).into_response();
    };

    let user = match auth::session_user(pool, &jar).await {
        Ok(Some(user)) if user.status == "approved" => user,
        Ok(Some(_)) => return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"account_not_approved"}))).into_response(),
        Ok(None) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"not_authenticated"}))).into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "product access session lookup failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"session_check_failed"}))).into_response();
        }
    };

    let consented = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM web_consents WHERE user_id = $1 AND terms_version = $2)",
    )
    .bind(&user.id)
    .bind(auth::TERMS_VERSION)
    .fetch_one(pool)
    .await;

    match consented {
        Ok(true) => next.run(request).await,
        Ok(false) => (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"terms_not_accepted"}))).into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "product access consent lookup failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"consent_check_failed"}))).into_response()
        }
    }
}
