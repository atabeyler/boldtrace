//! Bridges exchange-client's Redis publications into `MarketState`, and
//! notifies users the moment a registered `/alarm` threshold is crossed.

use futures_util::StreamExt;
use teloxide::prelude::*;
use teloxide::types::ChatId;

use crate::handlers::AppState;
use crate::i18n;

pub async fn run(redis_url: String, bot: Bot, state: AppState) -> redis::RedisResult<()> {
    let client = redis::Client::open(redis_url)?;
    let mut pubsub = client.get_async_pubsub().await?;
    pubsub.psubscribe("candles:*").await?;
    pubsub.psubscribe("orderbook:*").await?;
    pubsub.psubscribe("funding:*").await?;

    let mut messages = pubsub.on_message();
    while let Some(message) = messages.next().await {
        let channel = message.get_channel_name().to_string();
        let Ok(payload) = message.get_payload::<String>() else {
            continue;
        };

        let score = if let Some(_symbol_interval) = channel.strip_prefix("candles:") {
            serde_json::from_str::<shared::Candle>(&payload)
                .ok()
                .and_then(|candle| state.market_state.ingest_candle(candle))
        } else if channel.strip_prefix("orderbook:").is_some() {
            serde_json::from_str::<shared::OrderBookSnapshot>(&payload)
                .ok()
                .and_then(|snapshot| state.market_state.ingest_order_book(snapshot))
        } else if channel.strip_prefix("funding:").is_some() {
            serde_json::from_str::<shared::FundingRate>(&payload)
                .ok()
                .and_then(|rate| state.market_state.ingest_funding_rate(rate))
        } else {
            None
        };

        let Some(score) = score else { continue };
        for (telegram_id, threshold) in state.alarms.crossed(&score.symbol, score.value) {
            notify_alarm(&bot, &state, telegram_id, &score, threshold).await;
        }
    }

    Ok(())
}

async fn notify_alarm(bot: &Bot, state: &AppState, telegram_id: i64, score: &shared::Score, threshold: f64) {
    let lang = match state.user_store.get(telegram_id).await {
        Some(user) => user.language,
        None => "en".to_string(),
    };
    let body = i18n::t_args(
        &lang,
        "alarm-triggered",
        &[
            ("symbol", score.symbol.clone()),
            ("threshold", format!("{:.1}", threshold)),
            ("score", format!("{:.1}", score.value)),
        ],
    );
    let text = i18n::with_footer(&lang, &body);
    if let Err(err) = bot.send_message(ChatId(telegram_id), text).await {
        tracing::warn!(error = %err, "failed to send alarm notification");
    }
}
