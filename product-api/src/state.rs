use crate::email::EmailConfig;
use crate::live_store::LiveStore;
use crate::rate_limit::RateLimiter;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub live: Option<LiveStore>,
    pub pool: Option<PgPool>,
    pub secure_cookies: bool,
    pub auth_rate_limiter: Arc<RateLimiter>,
    pub email: EmailConfig,
}
