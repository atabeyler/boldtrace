//! Error type for the exchange-client crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExchangeClientError {
    #[error("websocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("json deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("http request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("connection closed by remote")]
    ConnectionClosed,
}

pub type Result<T> = std::result::Result<T, ExchangeClientError>;
