//! Telegram bot entry point: teloxide dispatcher wiring, consent-gated commands, and a background Redis subscriber feeding market state.
mod alarms;mod commands;mod consent;mod decision_history;mod decision_ledger;mod handlers;mod i18n;mod market_state;mod outcome_engine;mod postgres_user_store;mod redis_subscriber;mod user_store;
use std::sync::Arc;use teloxide::prelude::*;use crate::alarms::{AlarmRegistry,SmartAlertGate};use crate::commands::Command;use crate::decision_history::DecisionHistory;use crate::decision_ledger::DecisionLedger;use crate::handlers::{callback_handler,message_handler,AppState};use crate::market_state::MarketState;use crate::outcome_engine::OutcomeTracker;use crate::postgres_user_store::PostgresUserStore;use crate::user_store::{InMemoryUserStore,UserStore};
async fn build_stores()->(Arc<dyn UserStore>,Option<Arc<DecisionLedger>>){let Ok(database_url)=std::env::var("DATABASE_URL")else{tracing::info!("DATABASE_URL not set, using in-memory stores");return(Arc::new(InMemoryUserStore::default()),None);};match PostgresUserStore::connect(&database_url).await{Ok(store)=>{let ledger=Arc::new(DecisionLedger::new(store.pool()));(Arc::new(store),Some(ledger))}Err(err)=>{tracing::error!(error=%err,"failed to connect to Postgres, falling back to in-memory stores");(Arc::new(InMemoryUserStore::default()),None)}}}
#[tokio::main]async fn main(){let _=tracing_subscriber::fmt::try_init();let telegram_token=std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");let bot=Bot::new(telegram_token);let redis_url=std::env::var("REDIS_URL").unwrap_or_else(|_|"redis://127.0.0.1:6379".to_string());let weights=score_engine::ScoreWeights::from_env();let(user_store,decision_ledger)=build_stores().await;let state=AppState{user_store,market_state:Arc::new(MarketState::new(weights)),alarms:Arc::new(AlarmRegistry::default()),smart_alerts:Arc::new(SmartAlertGate::default()),decision_history:Arc::new(DecisionHistory::default()),decision_ledger,outcome_tracker:Arc::new(OutcomeTracker::default())};let subscriber_bot=bot.clone();let subscriber_state=state.clone();tokio::spawn(async move{
    // redis_subscriber::run only returns when its pub/sub stream ends (error
    // or graceful close); without a retry loop here, a single Redis hiccup
    // would permanently kill the market-data pipeline while the bot process
    // itself stays up and keeps answering Telegram commands, making the
    // outage silent instead of visible.
    let mut delay=std::time::Duration::from_secs(1);
    loop{
        match redis_subscriber::run(redis_url.clone(),subscriber_bot.clone(),subscriber_state.clone()).await{
            Ok(())=>tracing::warn!("redis subscriber stream ended, reconnecting"),
            Err(err)=>tracing::error!(error=%err,"redis subscriber exited, reconnecting"),
        }
        tokio::time::sleep(delay).await;
        delay=(delay*2).min(std::time::Duration::from_secs(30));
    }
});let handler=dptree::entry().branch(Update::filter_message().filter_command::<Command>().endpoint(message_handler)).branch(Update::filter_callback_query().endpoint(callback_handler));Dispatcher::builder(bot,handler).dependencies(dptree::deps![state]).enable_ctrlc_handler().build().dispatch().await;}
