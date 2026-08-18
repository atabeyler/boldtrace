mod adapter;
mod auth;
mod history;
mod live_store;
mod rate_limit;
mod state;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use live_store::LiveStore;
use serde::Serialize;
use shared::LiveIntelligence;
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use state::AppState;

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
    change24h: f64,
    regime: String,
    quality: f64,
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let live = env::var("REDIS_URL").ok().and_then(|u| LiveStore::new(&u).ok());

    let pool = match env::var("DATABASE_URL") {
        Ok(database_url) => match PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await
        {
            Ok(pool) => match sqlx::migrate!("../migrations").run(&pool).await {
                Ok(()) => Some(pool),
                Err(err) => {
                    tracing::error!(error=%err, "failed to run migrations, auth and history are unavailable");
                    None
                }
            },
            Err(err) => {
                tracing::error!(error=%err, "failed to connect to Postgres, auth and history are unavailable");
                None
            }
        },
        Err(_) => {
            tracing::info!("DATABASE_URL not set, auth and history are unavailable");
            None
        }
    };

    let secure_cookies = env::var("COOKIE_SECURE")
        .map(|v| v != "false")
        .unwrap_or(true);

    let state = AppState {
        live,
        pool,
        secure_cookies,
        auth_rate_limiter: std::sync::Arc::new(rate_limit::RateLimiter::default()),
    };

    let cors = env::var("WEB_ORIGIN").ok().map(|origin| {
        let origin: axum::http::HeaderValue = origin
            .parse()
            .expect("WEB_ORIGIN must be a valid origin, e.g. https://app.boldtrace.ai");
        CorsLayer::new()
            .allow_origin(origin)
            .allow_credentials(true)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    });

    let mut app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/intelligence/:symbol", get(intelligence))
        .route("/api/v1/history", get(history::history))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    if let Some(cors) = cors {
        app = app.layer(cors);
    }

    let web_dist = env::var("WEB_DIST_DIR").unwrap_or_else(|_| "web-dist".into());
    let index_path = format!("{web_dist}/index.html");
    if std::path::Path::new(&web_dist).is_dir() {
        let static_files = ServeDir::new(&web_dist).fallback(ServeFile::new(&index_path));
        app = app.fallback_service(static_files);
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
    let redis_ok = match s.live.as_ref() {
        Some(x) => x.ping().await,
        None => false,
    };
    let postgres_status = match s.pool.as_ref() {
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => "healthy",
            Err(_) => "degraded",
        },
        None => "offline",
    };
    Json(vec![
        ServiceHealth { name: "Product API".into(), status: "healthy".into(), freshness_ms: Some(0), latency_ms: Some(0) },
        ServiceHealth { name: "Redis".into(), status: if redis_ok { "healthy" } else { "degraded" }.into(), freshness_ms: None, latency_ms: None },
        ServiceHealth { name: "Postgres".into(), status: postgres_status.into(), freshness_ms: None, latency_ms: None },
    ])
}

async fn intelligence(State(s): State<AppState>, Path(symbol): Path<String>) -> impl IntoResponse {
    let symbol = symbol.to_uppercase();
    if symbol.len() > 24 || !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid symbol"}))).into_response();
    }
    let Some(store) = s.live.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"live intelligence store unavailable"}))).into_response();
    };
    let live = match store.intelligence(&symbol).await {
        Ok(Some(x)) => x,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"no fresh intelligence for symbol"}))).into_response(),
        Err(e) => {
            tracing::warn!(error=%e, %symbol, "live intelligence read failed");
            return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"live intelligence read failed"}))).into_response();
        }
    };
    Json(to_market(live)).into_response()
}

fn to_market(x: LiveIntelligence) -> MarketDecision {
    let s = &x.score;
    let engines = vec![
        EngineEvidence { name: "Order Book Imbalance".into(), score: s.order_book_imbalance, state: state_label(s.order_book_imbalance), weight: x.order_book_weight, reliability: "Adaptive".into() },
        EngineEvidence { name: "Volume Anomaly".into(), score: s.volume_anomaly, state: state_label(s.volume_anomaly), weight: x.volume_weight, reliability: "Adaptive".into() },
        EngineEvidence { name: "Funding Extreme".into(), score: s.funding_extreme, state: state_label(s.funding_extreme), weight: x.funding_weight, reliability: "Adaptive".into() },
        EngineEvidence { name: "RSI Divergence".into(), score: s.rsi_divergence, state: state_label(s.rsi_divergence), weight: x.rsi_weight, reliability: "Adaptive".into() },
    ];
    let decision = match x.decision.as_str() {
        "StrongLong" | "Long" => "LONG",
        "Short" | "StrongShort" => "SHORT",
        "NoTrade" => "NO TRADE",
        _ => "WATCH",
    }
    .into();
    let mut reasons = x.reasons;
    reasons.extend(x.warnings.into_iter().map(|w| format!("Warning: {w}")));
    MarketDecision {
        symbol: x.symbol,
        price: x.price,
        decision,
        confidence: x.confidence,
        risk: x.risk,
        change24h: 0.0,
        regime: x.regime.to_uppercase(),
        quality: (x.agreement * 0.6 + x.data_quality * 0.4).clamp(0.0, 100.0),
        engines,
        reasons,
        timestamp: x.timestamp.to_string(),
    }
}

fn state_label(v: f64) -> String {
    if v >= 70.0 { "High" } else if v >= 50.0 { "Moderate" } else { "Low" }.into()
}
