//! Telegram command and callback handlers. Consent is enforced once here
//! before any command other than `/start` executes.

use crate::alarms::{AlarmRegistry, SmartAlertGate};
use crate::commands::Command;
use crate::consent;
use crate::decision_history::DecisionHistory;
use crate::decision_ledger::DecisionLedger;
use crate::i18n;
use crate::market_state::MarketState;
use crate::outcome_engine::OutcomeTracker;
use crate::user_store::UserStore;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

#[derive(Clone)]
pub struct AppState {
    pub user_store: Arc<dyn UserStore>,
    pub market_state: Arc<MarketState>,
    pub alarms: Arc<AlarmRegistry>,
    pub smart_alerts: Arc<SmartAlertGate>,
    pub decision_history: Arc<DecisionHistory>,
    pub decision_ledger: Option<Arc<DecisionLedger>>,
    pub outcome_tracker: Arc<OutcomeTracker>,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn language_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(
        i18n::SUPPORTED_LANGUAGES
            .iter()
            .map(|code| {
                vec![InlineKeyboardButton::callback(
                    i18n::t("en", &format!("language-name-{code}")),
                    format!("lang:{code}"),
                )]
            })
            .collect::<Vec<_>>(),
    )
}

fn consent_keyboard(lang: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        i18n::t(lang, "consent-accept-button"),
        "consent:accept",
    )]])
}

async fn send(
    bot: &Bot,
    chat_id: ChatId,
    lang: &str,
    body: &str,
    keyboard: Option<InlineKeyboardMarkup>,
) {
    let mut request = bot.send_message(chat_id, i18n::with_footer(lang, body));
    if let Some(keyboard) = keyboard {
        request = request.reply_markup(keyboard);
    }
    if let Err(err) = request.await {
        tracing::warn!(error=%err, "failed to send message");
    }
}

async fn resolved_language(state: &AppState, id: i64, code: Option<&str>) -> String {
    match state.user_store.get(id).await {
        Some(user) => user.language,
        None => i18n::normalize_language(code),
    }
}

async fn send_consent_screen(bot: &Bot, id: ChatId, lang: &str) {
    send(
        bot,
        id,
        lang,
        &format!(
            "{}\n\n{}",
            i18n::t(lang, "consent-title"),
            i18n::t(lang, "consent-body")
        ),
        Some(consent_keyboard(lang)),
    )
    .await;
}

pub async fn message_handler(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: AppState,
) -> ResponseResult<()> {
    let chat = msg.chat.id;
    let id = chat.0;
    let lang = resolved_language(
        &state,
        id,
        msg.from.as_ref().and_then(|user| user.language_code.as_deref()),
    )
    .await;
    if !matches!(cmd, Command::Start)
        && !consent::has_current_consent(state.user_store.get(id).await.as_ref())
    {
        send_consent_screen(&bot, chat, &lang).await;
        return Ok(());
    }
    match cmd {
        Command::Start => handle_start(&bot, chat, &state, &lang).await,
        Command::Help => handle_help(&bot, chat, &lang).await,
        Command::Language => {
            send(
                &bot,
                chat,
                &lang,
                &i18n::t(&lang, "language-prompt"),
                Some(language_keyboard()),
            )
            .await
        }
        Command::Tara(symbol) => handle_tara(&bot, chat, &state, &lang, symbol).await,
        Command::History(symbol) => handle_history(&bot, chat, &state, &lang, symbol).await,
        Command::Performance(symbol) => {
            handle_performance(&bot, chat, &state, &lang, symbol).await
        }
        Command::Alarm { symbol, threshold } => {
            handle_alarm(&bot, chat, id, &state, &lang, symbol, threshold).await
        }
    }
    Ok(())
}

async fn handle_start(bot: &Bot, id: ChatId, state: &AppState, lang: &str) {
    if state.user_store.get(id.0).await.is_none() {
        state.user_store.set_language(id.0, lang).await;
    }
    send(
        bot,
        id,
        lang,
        &format!(
            "{}\n{}",
            i18n::t(lang, "welcome-greeting"),
            i18n::t(lang, "welcome-choose-language")
        ),
        Some(language_keyboard()),
    )
    .await;
}

async fn handle_help(bot: &Bot, id: ChatId, lang: &str) {
    let body = [
        i18n::t(lang, "help-title"),
        i18n::t(lang, "help-tara"),
        i18n::t(lang, "help-alarm"),
        i18n::t(lang, "help-language"),
        i18n::t(lang, "help-help"),
        "/history <SYMBOL>".into(),
        "/performance <SYMBOL>".into(),
    ]
    .join("\n");
    send(bot, id, lang, &body, None).await;
}

async fn handle_tara(
    bot: &Bot,
    id: ChatId,
    state: &AppState,
    lang: &str,
    symbol: String,
) {
    let symbol = symbol.trim().to_uppercase();
    // The subscriber writes only the fully calibrated, post-Risk-Guardian
    // snapshot into DecisionHistory. Never recompute a raw/pre-veto decision
    // on demand here, otherwise Telegram could disagree with Web/ledger.
    let record = state.decision_history.latest(&symbol, 1).into_iter().next();
    let Some(record) = record else {
        send(
            bot,
            id,
            lang,
            &i18n::t_args(lang, "tara-no-data", &[("symbol", symbol)]),
            None,
        )
        .await;
        return;
    };
    let body = format!(
        "{}\nDecision: {:?}\nScore: {:.1}\nConfidence: {:.1}\nRisk: {:.1}\nData quality: {:.1}\nAgreement: {:.1}\nReasons: {}\nWarnings: {}",
        record.symbol,
        record.decision,
        record.score,
        record.confidence,
        record.risk,
        record.data_quality,
        record.agreement,
        record.reasons.join(", "),
        record.warnings.join(", ")
    );
    send(bot, id, lang, &body, None).await;
}

async fn handle_history(
    bot: &Bot,
    id: ChatId,
    state: &AppState,
    lang: &str,
    symbol: String,
) {
    let symbol = symbol.trim().to_uppercase();
    if let Some(ledger) = state.decision_ledger.as_ref() {
        match ledger.replay(&symbol, 10).await {
            Ok(rows) if !rows.is_empty() => {
                let body = rows
                    .iter()
                    .map(|row| {
                        format!(
                            "{} | {} | {} | score {:.1} | conf {:.1} | {}",
                            row.decided_at_millis,
                            row.symbol,
                            row.decision,
                            row.score,
                            row.confidence,
                            row.rationale
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                send(bot, id, lang, &body, None).await;
                return;
            }
            Ok(_) => {}
            Err(err) => tracing::warn!(error=%err, "ledger replay failed"),
        }
    }
    let rows = state.decision_history.latest(&symbol, 10);
    let body = if rows.is_empty() {
        format!("No decision history for {symbol}")
    } else {
        rows.iter()
            .map(|row| {
                format!(
                    "{} | {:?} | score {:.1} | conf {:.1}",
                    row.timestamp, row.decision, row.score, row.confidence
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    send(bot, id, lang, &body, None).await;
}

async fn handle_performance(
    bot: &Bot,
    id: ChatId,
    state: &AppState,
    lang: &str,
    symbol: String,
) {
    let symbol = symbol.trim().to_uppercase();
    let Some(ledger) = state.decision_ledger.as_ref() else {
        send(
            bot,
            id,
            lang,
            "Persistent performance data requires DATABASE_URL.",
            None,
        )
        .await;
        return;
    };
    match ledger.performance(&symbol, 200).await {
        Ok(Some(performance)) => {
            send(
                bot,
                id,
                lang,
                &format!(
                    "{} performance\nSamples: {}\nAvg score: {:.1}\nAvg confidence: {:.1}\nAvg risk: {:.1}\nAvg data quality: {:.1}\nAvg agreement: {:.1}",
                    symbol,
                    performance.samples,
                    performance.avg_score,
                    performance.avg_confidence,
                    performance.avg_risk,
                    performance.avg_data_quality,
                    performance.avg_agreement
                ),
                None,
            )
            .await
        }
        Ok(None) => {
            send(
                bot,
                id,
                lang,
                &format!("No performance history for {symbol}"),
                None,
            )
            .await
        }
        Err(err) => {
            tracing::warn!(error=%err, "ledger performance failed");
            send(bot, id, lang, "Performance query failed.", None).await;
        }
    }
}

async fn handle_alarm(
    bot: &Bot,
    id: ChatId,
    telegram_id: i64,
    state: &AppState,
    lang: &str,
    symbol: String,
    threshold: String,
) {
    let symbol = symbol.trim().to_uppercase();
    let threshold = match threshold.trim().parse::<f64>() {
        Ok(value) if (0.0..=100.0).contains(&value) && !symbol.is_empty() => value,
        Ok(_) => {
            send(
                bot,
                id,
                lang,
                &i18n::t(lang, "alarm-invalid-threshold"),
                None,
            )
            .await;
            return;
        }
        Err(_) => {
            send(bot, id, lang, &i18n::t(lang, "alarm-usage"), None).await;
            return;
        }
    };
    state.alarms.set(telegram_id, &symbol, threshold);
    send(
        bot,
        id,
        lang,
        &i18n::t_args(
            lang,
            "alarm-set",
            &[
                ("symbol", symbol),
                ("threshold", format!("{threshold:.1}")),
            ],
        ),
        None,
    )
    .await;
}

pub async fn callback_handler(
    bot: Bot,
    query: CallbackQuery,
    state: AppState,
) -> ResponseResult<()> {
    let data = query.data.clone().unwrap_or_default();
    let Some(message) = query.regular_message() else {
        bot.answer_callback_query(query.id).await.ok();
        return Ok(());
    };
    let id = message.chat.id;
    let telegram_id = id.0;
    if let Some(code) = data.strip_prefix("lang:") {
        if i18n::is_supported(code) {
            state.user_store.set_language(telegram_id, code).await;
            send(
                &bot,
                id,
                code,
                &i18n::t_args(
                    code,
                    "language-changed",
                    &[("language", i18n::t(code, &format!("language-name-{code}")))],
                ),
                None,
            )
            .await;
            if !consent::has_current_consent(state.user_store.get(telegram_id).await.as_ref()) {
                send_consent_screen(&bot, id, code).await;
            }
        }
    } else if data == "consent:accept" {
        state
            .user_store
            .record_consent(telegram_id, consent::CURRENT_TERMS_VERSION, now_millis())
            .await;
        let lang = resolved_language(&state, telegram_id, None).await;
        handle_help(&bot, id, &lang).await;
    }
    bot.answer_callback_query(query.id).await.ok();
    Ok(())
}
