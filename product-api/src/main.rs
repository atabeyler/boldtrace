mod access;
mod adapter;
mod admin;
mod alerts;
mod auth;
mod email;
mod geoip;
mod history;
mod learning;
mod live_store;
mod performance;
mod rate_limit;
mod state;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use live_store::LiveStore;
use serde::Serialize;
use shared::LiveIntelligence;
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr, time::{SystemTime, UNIX_EPOCH}};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use state::AppState;

fn now_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineEvidence {
    name: String,
    score: f64,
    state: String,
    weight: f64,
    reliability: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MarketDecision {
    symbol: String,
    price: f64,
    decision: String,
    confidence: f64,
    risk: f64,
    change24h: Option<f64>,
    regime: String,
    quality: f64,
    freshness_ms: u64,
    engines: Vec<EngineEvidence>,
    reasons: Vec<String>,
    timestamp: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceHealth {
    name: String,
    status: String,
    freshness_ms: Option<u64>,
    latency_ms: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScannerEntry {
    symbol: String,
    status: String,
    market: Option<MarketDecision>,
}

fn scan_symbols() -> Option<Vec<String>> {
    let raw = env::var("SCAN_SYMBOLS").unwrap_or_else(|_| "BTCUSDT".into());
    if raw.trim().eq_ignore_ascii_case("ALL") {
        return None;
    }
    Some(raw.split(',').map(|s| s.trim().to_uppercase()).filter(|s| !s.is_empty()).collect())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let live = env::var("REDIS_URL").ok().and_then(|u| LiveStore::new(&u).ok());
    let pool = match env::var("DATABASE_URL") {
        Ok(database_url) => match PgPoolOptions::new().max_connections(10).connect(&database_url).await {
            Ok(pool) => match sqlx::migrate!("../migrations").run(&pool).await {
                Ok(()) => Some(pool),
                Err(err) => { tracing::error!(error=%err, "failed to run migrations, auth and history are unavailable"); None }
            },
            Err(err) => { tracing::error!(error=%err, "failed to connect to Postgres, auth and history are unavailable"); None }
        },
        Err(_) => { tracing::info!("DATABASE_URL not set, auth and history are unavailable"); None }
    };
    let secure_cookies = env::var("COOKIE_SECURE").map(|v| v != "false").unwrap_or(true);
    let state = AppState { live, pool, secure_cookies, auth_rate_limiter: std::sync::Arc::new(rate_limit::RateLimiter::default()), email: email::EmailConfig::from_env() };
    let cors = env::var("WEB_ORIGIN").ok().map(|origin| {
        let origin: axum::http::HeaderValue = origin.parse().expect("WEB_ORIGIN must be a valid origin, e.g. https://app.boldtrace.ai");
        CorsLayer::new().allow_origin(origin).allow_credentials(true).allow_methods(tower_http::cors::Any).allow_headers(tower_http::cors::Any)
    });

    let protected = Router::new()
        .route("/api/v1/intelligence/:symbol", get(intelligence))
        .route("/api/v1/scanner", get(scanner))
        .route("/api/v1/performance/:symbol", get(performance::performance))
        .route("/api/v1/learning/:symbol", get(learning::learning))
        .route("/api/v1/alerts", get(alerts::alerts))
        .route("/api/v1/history", get(history::history))
        .route_layer(middleware::from_fn_with_state(state.clone(), access::require_product_access));

    let mut app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/profile", axum::routing::patch(auth::update_profile))
        .route("/api/v1/auth/password", post(auth::change_password))
        .route("/api/v1/admin/pending-users", get(admin::list_pending))
        .route("/api/v1/admin/users", get(admin::list_all_users))
        .route("/api/v1/admin/users/:id/approve", post(admin::approve))
        .route("/api/v1/admin/users/:id/reject", post(admin::reject))
        .route("/api/v1/admin/location-alerts", get(admin::list_location_alerts))
        .route("/api/v1/admin/location-alerts/:id/allow", post(admin::allow_location))
        .merge(protected)
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    if let Some(cors) = cors { app = app.layer(cors); }
    let web_dist = env::var("WEB_DIST_DIR").unwrap_or_else(|_| "web-dist".into());
    let index_path = format!("{web_dist}/index.html");
    if std::path::Path::new(&web_dist).is_dir() {
        app = app.fallback_service(ServeDir::new(&web_dist).fallback(ServeFile::new(&index_path)));
    } else {
        tracing::info!(dir = %web_dist, "web build output not found, serving API only");
    }
    let port = env::var("PRODUCT_API_PORT").ok().and_then(|x| x.parse().ok()).unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind product api");
    tracing::info!(%addr, "Boldtrace Product API online");
    axum::serve(listener, app).await.expect("serve product api");
}

async fn health(State(s): State<AppState>) -> Json<Vec<ServiceHealth>> {
    let redis_started = std::time::Instant::now();
    let redis_ok = match s.live.as_ref() { Some(x) => x.ping().await, None => false };
    let redis_latency = s.live.as_ref().map(|_| redis_started.elapsed().as_millis() as u64);
    let pg_started = std::time::Instant::now();
    let postgres_status = match s.pool.as_ref() {
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await { Ok(_) => "healthy", Err(_) => "degraded" },
        None => "offline",
    };
    let postgres_latency = s.pool.as_ref().map(|_| pg_started.elapsed().as_millis() as u64);
    let (exchange_status, exchange_freshness_ms) = match s.live.as_ref() {
        Some(store) => {
            let reference = scan_symbols().and_then(|s| s.into_iter().next()).unwrap_or_else(|| "BTCUSDT".into());
            match store.intelligence_age_ms(&reference).await {
                Ok(Some(age_ms)) => (if age_ms < 60_000 { "healthy" } else { "degraded" }, Some(age_ms as u64)),
                Ok(None) => ("offline", None),
                Err(_) => ("degraded", None),
            }
        }
        None => ("offline", None),
    };
    Json(vec![
        ServiceHealth { name: "Product API".into(), status: "healthy".into(), freshness_ms: None, latency_ms: None },
        ServiceHealth { name: "Redis".into(), status: if redis_ok { "healthy" } else { "degraded" }.into(), freshness_ms: None, latency_ms: redis_latency },
        ServiceHealth { name: "Postgres".into(), status: postgres_status.into(), freshness_ms: None, latency_ms: postgres_latency },
        ServiceHealth { name: "Exchange Feed".into(), status: exchange_status.into(), freshness_ms: exchange_freshness_ms, latency_ms: None },
    ])
}

async fn scanner(State(s): State<AppState>) -> Json<Vec<ScannerEntry>> {
    let fixed = scan_symbols();
    let Some(store) = s.live.as_ref() else { return Json(fixed.unwrap_or_default().into_iter().map(|symbol| ScannerEntry { symbol, status: "unavailable".into(), market: None }).collect()); };
    let symbols = match fixed { Some(symbols) => symbols, None => store.live_symbols().await.unwrap_or_default() };
    let mut out = Vec::new();
    for symbol in symbols {
        let entry = match store.intelligence(&symbol).await {
            Ok(Some(live)) => ScannerEntry { symbol: symbol.clone(), status: if now_millis().saturating_sub(live.timestamp) < 60_000 { "live".into() } else { "stale".into() }, market: Some(to_market(live)) },
            _ => ScannerEntry { symbol, status: "unavailable".into(), market: None },
        };
        out.push(entry);
    }
    Json(out)
}

async fn intelligence(State(s): State<AppState>, Path(symbol): Path<String>) -> impl IntoResponse {
    let symbol = symbol.to_uppercase();
    if symbol.len() > 24 || !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid symbol"}))).into_response();
    }
    let Some(store) = s.live.as_ref() else { return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"live intelligence store unavailable"}))).into_response(); };
    match store.intelligence(&symbol).await {
        Ok(Some(x)) => Json(to_market(x)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"no fresh intelligence for symbol"}))).into_response(),
        Err(e) => { tracing::warn!(error=%e, %symbol, "live intelligence read failed"); (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"live intelligence read failed"}))).into_response() }
    }
}

fn to_market(x: LiveIntelligence) -> MarketDecision {
    let s = &x.score;
    let engines = vec![
        EngineEvidence { name: "Order Book Imbalance".into(), score: s.order_book_imbalance, state: state_label(s.order_book_imbalance), weight: x.order_book_weight, reliability: "Adaptive".into() },
        EngineEvidence { name: "Volume Anomaly".into(), score: s.volume_anomaly, state: state_label(s.volume_anomaly), weight: x.volume_weight, reliability: "Adaptive".into() },
        EngineEvidence { name: "Funding Extreme".into(), score: s.funding_extreme, state: state_label(s.funding_extreme), weight: x.funding_weight, reliability: "Adaptive".into() },
        EngineEvidence { name: "RSI Divergence".into(), score: s.rsi_divergence, state: state_label(s.rsi_divergence), weight: x.rsi_weight, reliability: "Adaptive".into() },
    ];
    let decision = match x.decision.as_str() { "StrongLong" | "Long" => "LONG", "Short" | "StrongShort" => "SHORT", "NoTrade" => "NO TRADE", _ => "WATCH" }.into();
    let mut reasons = x.reasons;
    reasons.extend(x.warnings.into_iter().map(|w| format!("Warning: {w}")));
    MarketDecision {
        symbol: x.symbol,
        price: x.price,
        decision,
        confidence: x.confidence,
        risk: x.risk,
        change24h: None,
        regime: x.regime.to_uppercase(),
        quality: (x.agreement * 0.6 + x.data_quality * 0.4).clamp(0.0, 100.0),
        freshness_ms: now_millis().saturating_sub(x.timestamp).max(0) as u64,
        engines,
        reasons,
        timestamp: x.timestamp.to_string(),
    }
}

fn state_label(v: f64) -> String {
    if v >= 70.0 { "High" } else if v >= 50.0 { "Moderate" } else { "Low" }.into()
}
