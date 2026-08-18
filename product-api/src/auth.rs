//! Server-enforced email/password authentication for the web product.
//! Sessions are HttpOnly cookies whose *hash* (not the raw bearer value) is
//! persisted, so a database leak alone cannot be used to impersonate a
//! live session.

use argon2::password_hash::rand_core::OsRng as PasswordOsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::http::StatusCode;
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
use time::{Duration, OffsetDateTime};

pub const TERMS_VERSION: &str = "2026-08-18";
const SESSION_COOKIE: &str = "bt_session";
const REMEMBER_DAYS: i64 = 30;
const DEFAULT_SESSION_HOURS: i64 = 12;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub user_code: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub language: String,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    user_code: String,
    email: String,
    first_name: String,
    last_name: String,
    password_hash: String,
    language: String,
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
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub language: Option<String>,
    pub terms_accepted: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
}

fn error(status: StatusCode, code: &'static str) -> (StatusCode, Json<ErrorBody>) {
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

fn generate_user_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut buf = [0u8; 6];
    OsRng.fill_bytes(&mut buf);
    let suffix: String = buf
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect();
    format!("BT-{suffix}")
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

fn require_pool(state: &AppState) -> Result<PgPool, (StatusCode, Json<ErrorBody>)> {
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
    let first_name = req.first_name.trim().to_string();
    let last_name = req.last_name.trim().to_string();
    if email.is_empty()
        || !email.contains('@')
        || first_name.is_empty()
        || last_name.is_empty()
        || req.password.len() < 8
    {
        return error(StatusCode::BAD_REQUEST, "invalid_input").into_response();
    }
    if !req.terms_accepted {
        return error(StatusCode::BAD_REQUEST, "terms_not_accepted").into_response();
    }
    let Some(password_hash) = hash_password(&req.password) else {
        return error(StatusCode::INTERNAL_SERVER_ERROR, "hash_failed").into_response();
    };
    let language = req.language.unwrap_or_else(|| "en".into());
    let id = random_hex(16);

    let mut attempts = 0;
    let row: Result<UserRow, sqlx::Error> = loop {
        attempts += 1;
        let user_code = generate_user_code();
        let result = sqlx::query_as::<_, UserRow>(
            "INSERT INTO web_users (id, user_code, email, first_name, last_name, password_hash, language) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             RETURNING id, user_code, email, first_name, last_name, password_hash, language",
        )
        .bind(&id)
        .bind(&user_code)
        .bind(&email)
        .bind(&first_name)
        .bind(&last_name)
        .bind(&password_hash)
        .bind(&language)
        .fetch_one(&pool)
        .await;
        match result {
            Ok(row) => break Ok(row),
            Err(sqlx::Error::Database(db_err))
                if db_err.constraint() == Some("web_users_user_code_key") && attempts < 5 =>
            {
                continue
            }
            Err(err) => break Err(err),
        }
    };

    let row = match row {
        Ok(row) => row,
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("web_users_email_key") => {
            return error(StatusCode::CONFLICT, "email_taken").into_response();
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
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let pool = match require_pool(&state) {
        Ok(pool) => pool,
        Err(err) => return err.into_response(),
    };
    let email = req.email.trim().to_lowercase();
    let remember_me = req.remember_me.unwrap_or(false);
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, user_code, email, first_name, last_name, password_hash, language FROM web_users WHERE email = $1",
    )
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

pub async fn me(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let pool = match require_pool(&state) {
        Ok(pool) => pool,
        Err(err) => return err.into_response(),
    };
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return error(StatusCode::UNAUTHORIZED, "not_authenticated").into_response();
    };
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT u.id, u.user_code, u.email, u.first_name, u.last_name, u.password_hash, u.language \
         FROM web_sessions s JOIN web_users u ON u.id = s.user_id \
         WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(hash_token(cookie.value()))
    .fetch_optional(&pool)
    .await;

    match row {
        Ok(Some(row)) => Json(AccountView::from(row)).into_response(),
        Ok(None) => error(StatusCode::UNAUTHORIZED, "not_authenticated").into_response(),
        Err(err) => {
            tracing::warn!(error=%err, "session lookup failed");
            error(StatusCode::INTERNAL_SERVER_ERROR, "session_check_failed").into_response()
        }
    }
}
