//! Telegram bot entry point: teloxide dispatcher wiring, consent-gated
//! commands, and a background Redis subscriber feeding market state.

mod alarms;
mod commands;
mod consent;
mod handlers;
mod i18n;
mod market_state;
mod redis_subscriber;
mod user_store;

use std::sync::Arc;

use teloxide::prelude::*;

use crate::alarms::AlarmRegistry;
use crate::commands::Command;
use crate::handlers::{callback_handler, message_handler, AppState};
use crate::market_state::MarketState;
use crate::user_store::InMemoryUserStore;

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt::try_init();

    let bot = Bot::from_env();
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let weights = score_engine::ScoreWeights::from_env();

    let state = AppState {
        user_store: Arc::new(InMemoryUserStore::default()),
        market_state: Arc::new(MarketState::new(weights)),
        alarms: Arc::new(AlarmRegistry::default()),
    };

    let subscriber_bot = bot.clone();
    let subscriber_state = state.clone();
    tokio::spawn(async move {
        if let Err(err) = redis_subscriber::run(redis_url, subscriber_bot, subscriber_state).await {
            tracing::error!(error = %err, "redis subscriber exited");
        }
    });

    let handler = dptree::entry()
        .branch(Update::filter_message().filter_command::<Command>().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
