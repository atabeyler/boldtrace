//! Server-enforced email/password authentication for the web product.
//! Sessions are HttpOnly cookies whose *hash* (not the raw bearer value) is
//! persisted, so a database leak alone cannot be used to impersonate a
//! live session.

use argon2::password_hash::rand_core::OsRng as PasswordOsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::time::Duration as StdDuration;
use time::{Duration, OffsetDateTime};

pub const TERMS_VERSION: &str = "2026-08-18";
const SESSION_COOKIE: &str = "bt_session";
const REMEMBER_DAYS: i64 = 30;
const DEFAULT_SESSION_HOURS: i64 = 12;
const LOGIN_MAX_ATTEMPTS: usize = 8;
const LOGIN_WINDOW: StdDuration = StdDuration::from_secs(15 * 60);
const REGISTER_MAX_ATTEMPTS: usize = 5;
const REGISTER_WINDOW: StdDuration = StdDuration::from_secs(60 * 60);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub user_code: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub language: String,
    pub status: String,
    pub is_admin: bool,
}

#[derive(sqlx::FromRow)]
pub(crate) struct UserRow {
    pub(crate) id: String,
    pub(crate) user_code: String,
    pub(crate) email: String,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) password_hash: String,
    pub(crate) language: String,
    pub(crate) status: String,
    pub(crate) is_admin: bool,
    pub(crate) country: String,
    pub(crate) location_override_until: Option<OffsetDateTime>,
}

impl From<UserRow> for AccountView {
    fn from(r: UserRow) -> Self {
        Self {
            id: r.id,
            user_code: r.user_code,
            email: r.email,
            first_name: r.first_name,
            last_name: r.last_name,
            language: r.language,
            status: r.status,
            is_admin: r.is_admin,
        }
    }
}

const USER_ROW_COLUMNS: &str =
    "id, user_code, email, first_name, last_name, password_hash, language, status, is_admin, country, location_override_until";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub first_name: String,
    pub last_name: String,
    pub user_code: String,
    pub country: String,
    pub national_id: String,
    pub email: String,
    pub password: String,
    pub language: Option<String>,
    pub terms_accepted: bool,
}

fn is_valid_user_code(code: &str) -> bool {
    (4..=20).contains(&code.len()) && code.chars().all(|c| c.is_ascii_alphanumeric())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
    /// Optional browser Geolocation API coordinates. Purely informational —
    /// recorded alongside a location alert for the admin to see, never used
    /// to enforce the country check itself since it's a client-supplied,
    /// spoofable signal. The IP-derived country is what's enforced.
    pub browser_lat: Option<f64>,
    pub browser_lon: Option<f64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorBody {
    error: &'static str,
}

pub(crate) fn error(status: StatusCode, code: &'static str) -> (StatusCode, Json<ErrorBody>) {
    (status, Json(ErrorBody { error: code }))
}

fn hash_password(password: &str) -> Option<String> {
    let salt = SaltString::generate(&mut PasswordOsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .ok()
        .map(|h| h.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn session_cookie(secure: bool, token: String, remember_me: bool) -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(secure);
    if remember_me {
        cookie.set_max_age(Duration::days(REMEMBER_DAYS));
    }
    cookie
}

pub(crate) fn require_pool(state: &AppState) -> Result<PgPool, (StatusCode, Json<ErrorBody>)> {
    state
        .pool
        .clone()
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))
}

async fn create_session(
    pool: &PgPool,
    user_id: &str,
    remember_me: bool,
) -> Result<String, sqlx::Error> {
    let token = random_hex(32);
    let expires_at = OffsetDateTime::now_utc()
        + if remember_me {
            Duration::days(REMEMBER_DAYS)
        } else {
            Duration::hours(DEFAULT_SESSION_HOURS)
        };
    sqlx::query("INSERT INTO web_sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(token)
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let pool = match require_pool(&state) {
        Ok(pool) => pool,
        Err(err) => return err.into_response(),
    };
    let email = req.email.trim().to_lowercase();
    if !state
        .auth_rate_limiter
        .check(&format!("register:{email}"), REGISTER_MAX_ATTEMPTS, REGISTER_WINDOW)
    {
        return error(StatusCode::TOO_MANY_REQUESTS, "rate_limited").into_response();
    }
    let first_name = req.first_name.trim().to_string();
    let last_name = req.last_name.trim().to_string();
    let country = req.country.trim().to_string();
    let national_id = req.national_id.trim().to_string();
    let user_code = req.user_code.trim().to_uppercase();
    if email.is_empty()
        || !email.contains('@')
        || first_name.is_empty()
        || last_name.is_empty()
        || country.is_empty()
        || national_id.is_empty()
        || req.password.len() < 8
    {
        return error(StatusCode::BAD_REQUEST, "invalid_input").into_response();
    }
    if !is_valid_user_code(&user_code) {
        return error(StatusCode::BAD_REQUEST, "invalid_user_code").into_response();
    }
    if !req.terms_accepted {
        return error(StatusCode::BAD_REQUEST, "terms_not_accepted").into_response();
    }
    let Some(password_hash) = hash_password(&req.password) else {
        return error(StatusCode::INTERNAL_SERVER_ERROR, "hash_failed").into_response();
    };
    let language = req.language.unwrap_or_else(|| "en".into());
    let id = random_hex(16);

    // A registration matching ADMIN_BOOTSTRAP_EMAIL is auto-approved and
    // made an admin, so the very first admin account can be created without
    // any prior admin existing to approve it.
    let is_bootstrap_admin = std::env::var("ADMIN_BOOTSTRAP_EMAIL")
        .map(|bootstrap| bootstrap.trim().to_lowercase() == email)
        .unwrap_or(false);
    let status = if is_bootstrap_admin { "approved" } else { "pending" };

    let result = sqlx::query_as::<_, UserRow>(&format!(
        "INSERT INTO web_users (id, user_code, email, first_name, last_name, password_hash, language, status, is_admin, country, national_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         RETURNING {USER_ROW_COLUMNS}"
    ))
    .bind(&id)
    .bind(&user_code)
    .bind(&email)
    .bind(&first_name)
    .bind(&last_name)
    .bind(&password_hash)
    .bind(&language)
    .bind(status)
    .bind(is_bootstrap_admin)
    .bind(&country)
    .bind(&national_id)
    .fetch_one(&pool)
    .await;

    let row = match result {
        Ok(row) => row,
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("web_users_email_key") => {
            return error(StatusCode::CONFLICT, "email_taken").into_response();
        }
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("web_users_user_code_key") => {
            return error(StatusCode::CONFLICT, "user_code_taken").into_response();
        }
        Err(err) => {
            tracing::warn!(error=%err, "failed to create web account");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "registration_failed").into_response();
        }
    };

    let consented_at = OffsetDateTime::now_utc().unix_timestamp() * 1000;
    if let Err(err) = sqlx::query(
        "INSERT INTO web_consents (user_id, terms_version, consented_at_millis) VALUES ($1, $2, $3)",
    )
    .bind(&row.id)
    .bind(TERMS_VERSION)
    .bind(consented_at)
    .execute(&pool)
    .await
    {
        tracing::warn!(error=%err, "failed to record web consent");
    }

    if row.status != "approved" {
        // Pending accounts can't sign in yet, so no session is issued.
        crate::email::notify_admin_new_registration(
            &state.email,
            &row.first_name,
            &row.last_name,
            &row.email,
            &row.user_code,
        )
        .await;
        return (StatusCode::CREATED, Json(AccountView::from(row))).into_response();
    }

    let token = match create_session(&pool, &row.id, false).await {
        Ok(token) => token,
        Err(err) => {
            tracing::warn!(error=%err, "failed to create session after registration");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "registration_failed").into_response();
        }
    };
    let jar = jar.add(session_cookie(state.secure_cookies, token, false));
    (StatusCode::CREATED, jar, Json(AccountView::from(row))).into_response()
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let pool = match require_pool(&state) {
        Ok(pool) => pool,
        Err(err) => return err.into_response(),
    };
    let email = req.email.trim().to_lowercase();
    if !state
        .auth_rate_limiter
        .check(&format!("login:{email}"), LOGIN_MAX_ATTEMPTS, LOGIN_WINDOW)
    {
        return error(StatusCode::TOO_MANY_REQUESTS, "rate_limited").into_response();
    }
    let remember_me = req.remember_me.unwrap_or(false);
    let row = sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {USER_ROW_COLUMNS} FROM web_users WHERE email = $1"
    ))
    .bind(&email)
    .fetch_optional(&pool)
    .await;

    let row = match row {
        Ok(row) => row,
        Err(err) => {
            tracing::warn!(error=%err, "login lookup failed");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "login_failed").into_response();
        }
    };

    // Always run a verification so a missing account doesn't respond
    // measurably faster than a wrong password (basic timing-side-channel
    // hygiene); the dummy hash never matches any real password.
    const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHR2YWx1ZQ$Y5s+3d1n2mR3o8m3lQyN4dQY8FhQ0h4mQe6MYd3qXyE";
    let ok = match row.as_ref() {
        Some(user) => verify_password(&req.password, &user.password_hash),
        None => {
            verify_password(&req.password, DUMMY_HASH);
            false
        }
    };
    let Some(row) = row.filter(|_| ok) else {
        return error(StatusCode::UNAUTHORIZED, "invalid_credentials").into_response();
    };
    match row.status.as_str() {
        "pending" => return error(StatusCode::FORBIDDEN, "account_pending").into_response(),
        "rejected" => return error(StatusCode::FORBIDDEN, "account_rejected").into_response(),
        _ => {}
    }

    let has_override = row
        .location_override_until
        .map(|until| OffsetDateTime::now_utc() < until)
        .unwrap_or(false);
    if !has_override && !row.country.is_empty() {
        if let Some(ip) = crate::geoip::client_ip(&headers) {
            if let Some(detected) = crate::geoip::lookup_country(&ip).await {
                if detected != row.country {
                    let alert_id = random_hex(16);
                    if let Err(err) = sqlx::query(
                        "INSERT INTO login_location_alerts \
                         (id, user_id, email, expected_country, detected_country, ip, browser_lat, browser_lon) \
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                    )
                    .bind(&alert_id)
                    .bind(&row.id)
                    .bind(&row.email)
                    .bind(&row.country)
                    .bind(&detected)
                    .bind(&ip)
                    .bind(req.browser_lat)
                    .bind(req.browser_lon)
                    .execute(&pool)
                    .await
                    {
                        tracing::warn!(error=%err, "failed to record login location alert");
                    }
                    crate::email::notify_admin_location_mismatch(
                        &state.email,
                        &row.first_name,
                        &row.last_name,
                        &row.email,
                        &row.country,
                        &detected,
                        &ip,
                    )
                    .await;
                    return error(StatusCode::FORBIDDEN, "location_mismatch").into_response();
                }
            }
            // Lookup failed (provider down, etc.): fail open, proceed with login.
        }
        // No IP resolvable (e.g. local dev behind no proxy): fail open.
    }

    let token = match create_session(&pool, &row.id, remember_me).await {
        Ok(token) => token,
        Err(err) => {
            tracing::warn!(error=%err, "failed to create session");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "login_failed").into_response();
        }
    };
    let jar = jar.add(session_cookie(state.secure_cookies, token, remember_me));
    (StatusCode::OK, jar, Json(AccountView::from(row))).into_response()
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if let (Some(pool), Some(cookie)) = (state.pool.as_ref(), jar.get(SESSION_COOKIE)) {
        let _ = sqlx::query("DELETE FROM web_sessions WHERE token_hash = $1")
            .bind(hash_token(cookie.value()))
            .execute(pool)
            .await;
    }
    let mut removal = Cookie::from(SESSION_COOKIE);
    removal.set_path("/");
    removal.set_secure(state.secure_cookies);
    let jar = jar.remove(removal);
    (StatusCode::NO_CONTENT, jar)
}

/// Resolves the caller's session cookie to their user row, if any live
/// session matches. Shared by `me` and the admin endpoints, which both need
/// to know who's asking before answering.
pub(crate) async fn session_user(
    pool: &PgPool,
    jar: &CookieJar,
) -> Result<Option<UserRow>, sqlx::Error> {
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Ok(None);
    };
    let columns = USER_ROW_COLUMNS
        .split(", ")
        .map(|c| format!("u.{c}"))
        .collect::<Vec<_>>()
        .join(", ");
    sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {columns} \
         FROM web_sessions s JOIN web_users u ON u.id = s.user_id \
         WHERE s.token_hash = $1 AND s.expires_at > now()"
    ))
    .bind(hash_token(cookie.value()))
    .fetch_optional(pool)
    .await
}

pub async fn me(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let pool = match require_pool(&state) {
        Ok(pool) => pool,
        Err(err) => return err.into_response(),
    };
    match session_user(&pool, &jar).await {
        Ok(Some(row)) => Json(AccountView::from(row)).into_response(),
        Ok(None) => error(StatusCode::UNAUTHORIZED, "not_authenticated").into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "session lookup failed");
            error(StatusCode::INTERNAL_SERVER_ERROR, "session_check_failed").into_response()
        }
    }
}
