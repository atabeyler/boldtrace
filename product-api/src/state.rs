use crate::live_store::LiveStore;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub live: Option<LiveStore>,
    pub pool: Option<PgPool>,
    pub secure_cookies: bool,
}
