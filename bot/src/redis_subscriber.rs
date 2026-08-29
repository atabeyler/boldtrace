//! Bridges exchange-client Redis publications into live market state.

use crate::alarms::AlertPolicy;
use crate::handlers::AppState;
use crate::i18n;
use crate::market_state::SpecializedSnapshot;
use futures_util::StreamExt;
use redis::AsyncCommands;
use score_engine::{calibrate_confidence, Decision, PerformanceFeedback, SweepSide};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use teloxide::prelude::*;
use teloxide::types::ChatId;

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub async fn run(redis_url: String, bot: Bot, state: AppState) -> redis::RedisResult<()> {
    let client = redis::Client::open(redis_url)?;
    let publisher = client.clone();
    let mut pubsub = client.get_async_pubsub().await?;
    pubsub.psubscribe("candles:*").await?;
    pubsub.psubscribe("orderbook:*").await?;
    pubsub.psubscribe("funding:*").await?;
    pubsub.psubscribe("open_interest:*").await?;
    tracing::info!("redis subscriber connected, listening for market data");

    let mut messages = pubsub.on_message();
    let mut logged_first_score = false;
    let mut seen_prefixes: HashSet<&'static str> = HashSet::new();

    while let Some(message) = messages.next().await {
        let channel = message.get_channel_name().to_string();
        for prefix in ["candles:", "orderbook:", "funding:", "open_interest:"] {
            if channel.starts_with(prefix) && seen_prefixes.insert(prefix) {
                tracing::info!(%channel, "received first message on this channel type since connecting");
            }
        }
        let Ok(payload) = message.get_payload::<String>() else {
            continue;
        };

        let candle = if channel.starts_with("candles:") {
            match serde_json::from_str::<shared::Candle>(&payload) {
                Ok(candle) => Some(candle),
                Err(err) => {
                    tracing::warn!(error=%err, %channel, "failed to parse candle payload");
                    None
                }
            }
        } else {
            None
        };
        // Only the closed primary 1m series is a statistical decision sample.
        // Depth/funding/OI and 5m updates still refresh live intelligence but
        // cannot inflate the ledger or realized-outcome sample count.
        let persist_decision_tick = candle
            .as_ref()
            .is_some_and(|candle| candle.interval == "1m");

        if let Some(candle) = candle.as_ref() {
            let outcomes = state
                .outcome_tracker
                .evaluate_price(&candle.symbol, candle.close, candle.close_time);
            let mut learned = false;
            for outcome in outcomes {
                if let Some(ledger) = state.decision_ledger.as_ref() {
                    if let Err(err) = ledger.append_outcome(&outcome).await {
                        tracing::warn!(error=%err, symbol=%outcome.symbol, horizon=outcome.horizon_minutes, "failed to persist decision outcome");
                    } else if outcome.horizon_minutes == 60 {
                        learned = true;
                    }
                }
                tracing::info!(symbol=%outcome.symbol, horizon=outcome.horizon_minutes, correct=outcome.correct, directional_return=outcome.directional_return_pct, "decision outcome evaluated");
            }
            if learned {
                refresh_adaptive_weights(&state, &candle.symbol).await;
            }
        }

        let score = if let Some(candle) = candle {
            state.market_state.ingest_candle(candle)
        } else if channel.starts_with("orderbook:") {
            serde_json::from_str::<shared::OrderBookSnapshot>(&payload)
                .ok()
                .and_then(|snapshot| state.market_state.ingest_order_book(snapshot))
        } else if channel.starts_with("funding:") {
            serde_json::from_str::<shared::FundingRate>(&payload)
                .ok()
                .and_then(|rate| state.market_state.ingest_funding_rate(rate))
        } else if channel.starts_with("open_interest:") {
            serde_json::from_str::<shared::OpenInterest>(&payload)
                .ok()
                .and_then(|oi| state.market_state.ingest_open_interest(oi))
        } else {
            None
        };
        let Some(score) = score else {
            continue;
        };

        if !logged_first_score {
            logged_first_score = true;
            tracing::info!(symbol=%score.symbol, value=score.value, "computed first composite score since connecting");
        }

        for (telegram_id, threshold) in state.alarms.crossed(&score.symbol, score.value) {
            notify_alarm(&bot, &state, telegram_id, &score, threshold).await;
        }

        let Some(raw_snapshot) = state.market_state.intelligence(&score.symbol) else {
            continue;
        };
        let (calibrated, policy) = calibration_and_policy(&state, &raw_snapshot).await;
        let specialized = state
            .market_state
            .specialized(&raw_snapshot.symbol, raw_snapshot.decision.decision);

        let mut final_snapshot = raw_snapshot.clone();
        final_snapshot.decision.confidence = calibrated;
        if let Some(specialized_snapshot) = specialized {
            let gate_warnings =
                apply_specialized_gate(&mut final_snapshot.decision, specialized_snapshot);
            final_snapshot.explanation.warnings.extend(gate_warnings);
        }

        // Runtime history keeps the latest final state for interactive scans.
        state.decision_history.push(&final_snapshot);
        if persist_decision_tick {
            if let Some(ledger) = state.decision_ledger.as_ref() {
                if let Err(err) = ledger.append(&final_snapshot).await {
                    tracing::warn!(error=%err, symbol=%final_snapshot.symbol, "failed to persist final intelligence decision");
                }
            }
            let price = state
                .market_state
                .latest_price(&final_snapshot.symbol)
                .unwrap_or_default();
            if price > 0.0 {
                state.outcome_tracker.register(
                    &final_snapshot.symbol,
                    final_snapshot.decision.decision,
                    final_snapshot.timestamp,
                    price,
                );
            }
        }

        let price = state
            .market_state
            .latest_price(&final_snapshot.symbol)
            .unwrap_or_default();
        publish_live(&publisher, &state, &final_snapshot, price).await;

        if let Some(alert) = state.smart_alerts.evaluate_with_policy(
            &final_snapshot.symbol,
            &final_snapshot.decision,
            now_millis(),
            policy,
        ) {
            for telegram_id in state.alarms.subscribers(&final_snapshot.symbol) {
                notify_smart_alert(
                    &bot,
                    &state,
                    telegram_id,
                    &final_snapshot,
                    specialized,
                    policy,
                )
                .await;
            }
            tracing::info!(symbol=%alert.symbol, decision=?alert.decision, confidence=alert.confidence, risk=alert.risk, "adaptive smart intelligence alert delivered");
        }
    }
    Ok(())
}

async fn publish_live(
    client: &redis::Client,
    state: &AppState,
    snapshot: &score_engine::IntelligenceSnapshot,
    price: f64,
) {
    let weights = state.market_state.adaptive_weights(&snapshot.symbol);
    let regime = state
        .market_state
        .regime(&snapshot.symbol)
        .map(|regime| format!("{:?}", regime))
        .unwrap_or_else(|| "Unknown".into());
    let live = shared::LiveIntelligence {
        symbol: snapshot.symbol.clone(),
        timestamp: now_millis(),
        price,
        score: shared::Score {
            symbol: snapshot.symbol.clone(),
            timestamp: snapshot.timestamp,
            value: snapshot.score,
            volume_anomaly: snapshot.components.volume_anomaly,
            funding_extreme: snapshot.components.funding_extreme,
            order_book_imbalance: snapshot.components.order_book_imbalance,
            rsi_divergence: snapshot.components.rsi_divergence,
        },
        decision: format!("{:?}", snapshot.decision.decision),
        confidence: snapshot.decision.confidence,
        risk: snapshot.decision.risk,
        data_quality: snapshot.data_quality,
        agreement: snapshot.agreement,
        regime,
        volume_weight: weights.volume_anomaly,
        funding_weight: weights.funding_extreme,
        order_book_weight: weights.order_book_imbalance,
        rsi_weight: weights.rsi_divergence,
        reasons: snapshot.explanation.reasons.clone(),
        warnings: snapshot.explanation.warnings.clone(),
    };
    let raw = match serde_json::to_string(&live) {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!(error=%err, symbol=%snapshot.symbol, "failed to serialize live intelligence");
            return;
        }
    };
    let mut connection = match client.get_multiplexed_async_connection().await {
        Ok(connection) => connection,
        Err(err) => {
            tracing::warn!(error=%err, symbol=%snapshot.symbol, "failed to open Redis connection to publish live intelligence");
            return;
        }
    };
    let key = format!("intelligence:{}", snapshot.symbol);
    match connection.set_ex::<_, _, ()>(key, raw, 120).await {
        Ok(()) => tracing::debug!(symbol=%snapshot.symbol, "published final live intelligence"),
        Err(err) => tracing::warn!(error=%err, symbol=%snapshot.symbol, "failed to publish product intelligence"),
    }
}

async fn refresh_adaptive_weights(state: &AppState, symbol: &str) {
    let Some(ledger) = state.decision_ledger.as_ref() else {
        return;
    };
    match ledger.signal_reliability(symbol, 500).await {
        Ok(reliability) => {
            let weights = state.market_state.apply_reliability(symbol, reliability);
            tracing::info!(
                symbol,
                samples=reliability.samples,
                volume_reliability=reliability.volume,
                funding_reliability=reliability.funding,
                order_book_reliability=reliability.order_book,
                rsi_reliability=reliability.rsi,
                volume_weight=weights.volume_anomaly,
                funding_weight=weights.funding_extreme,
                order_book_weight=weights.order_book_imbalance,
                rsi_weight=weights.rsi_divergence,
                "adaptive weights refreshed from realized outcomes"
            );
        }
        Err(err) => tracing::warn!(error=%err, symbol, "failed to refresh adaptive weights"),
    }
}

fn apply_specialized_gate(
    decision: &mut score_engine::MetaDecision,
    specialized: SpecializedSnapshot,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if specialized.conflict.force_no_trade {
        decision.decision = Decision::NoTrade;
        decision.confidence = decision.confidence.min(50.0);
        warnings.push("signal_conflict_veto".into());
        return warnings;
    }

    let sweep_against_decision = matches!(
        (decision.decision, specialized.sweep.side),
        (Decision::StrongLong | Decision::Long, SweepSide::Highs)
            | (Decision::StrongShort | Decision::Short, SweepSide::Lows)
    );
    let sweep_penalty = if sweep_against_decision {
        specialized.sweep.score * 0.12
    } else {
        0.0
    };
    let risk_penalty = specialized.shock.score * 0.18
        + specialized.derivatives_stress.score * 0.22
        + specialized.regime_transition.score * 0.08
        + specialized.cross_market.score * 0.07
        + sweep_penalty;
    decision.risk = (decision.risk + risk_penalty).clamp(0.0, 100.0);

    if specialized.shock.score >= 85.0 {
        warnings.push("market_shock_veto".into());
    }
    if specialized.derivatives_stress.score >= 90.0 {
        warnings.push("derivatives_stress_veto".into());
    }
    if specialized.regime_transition.score >= 90.0 {
        warnings.push("regime_transition_veto".into());
    }
    if sweep_against_decision && specialized.sweep.score >= 85.0 {
        warnings.push("liquidity_sweep_veto".into());
    }
    if decision.risk >= 90.0 {
        warnings.push("aggregate_risk_veto".into());
    }
    if !warnings.is_empty() {
        decision.decision = Decision::NoTrade;
        decision.confidence = decision.confidence.min(55.0);
    }
    warnings
}

async fn calibration_and_policy(
    state: &AppState,
    snapshot: &score_engine::IntelligenceSnapshot,
) -> (f64, AlertPolicy) {
    let Some(ledger) = state.decision_ledger.as_ref() else {
        return (snapshot.decision.confidence, AlertPolicy::default());
    };
    let baseline = ledger.performance(&snapshot.symbol, 200).await.ok().flatten();
    let outcome15 = ledger.outcome_performance(&snapshot.symbol, 15, 200).await.ok().flatten();
    let outcome60 = ledger.outcome_performance(&snapshot.symbol, 60, 200).await.ok().flatten();
    let outcome240 = ledger.outcome_performance(&snapshot.symbol, 240, 100).await.ok().flatten();
    let mut confidence = if let Some(performance) = baseline.as_ref() {
        calibrate_confidence(
            snapshot.decision.confidence,
            PerformanceFeedback {
                samples: performance.samples,
                avg_confidence: performance.avg_confidence,
                avg_risk: performance.avg_risk,
                avg_data_quality: performance.avg_data_quality,
                avg_agreement: performance.avg_agreement,
            },
        )
        .calibrated_confidence
    } else {
        snapshot.decision.confidence
    };
    let mut weighted_win = 0.0;
    let mut weight = 0.0;
    let mut realized_samples = 0usize;
    for (performance, horizon_weight) in [
        (outcome15.as_ref(), 0.25),
        (outcome60.as_ref(), 0.45),
        (outcome240.as_ref(), 0.30),
    ] {
        if let Some(performance) = performance {
            if performance.samples >= 10 {
                weighted_win += performance.win_rate * horizon_weight;
                weight += horizon_weight;
                realized_samples += performance.samples;
            }
        }
    }
    if weight > 0.0 {
        let win_rate = weighted_win / weight;
        let reliability = (realized_samples as f64 / 150.0).clamp(0.0, 1.0);
        let adjustment = ((win_rate - 50.0) * 0.35 * reliability).clamp(-15.0, 15.0);
        confidence = (confidence + adjustment).clamp(0.0, 100.0);
    }
    let policy = if let Some(performance) = baseline {
        AlertPolicy::adaptive(
            performance.samples,
            performance.avg_risk,
            performance.avg_data_quality,
            performance.avg_agreement,
        )
    } else {
        AlertPolicy::default()
    };
    (confidence, policy)
}

async fn notify_alarm(
    bot: &Bot,
    state: &AppState,
    telegram_id: i64,
    score: &shared::Score,
    threshold: f64,
) {
    let lang = match state.user_store.get(telegram_id).await {
        Some(user) => user.language,
        None => "en".into(),
    };
    let body = i18n::t_args(
        &lang,
        "alarm-triggered",
        &[
            ("symbol", score.symbol.clone()),
            ("threshold", format!("{threshold:.1}")),
            ("score", format!("{:.1}", score.value)),
        ],
    );
    if let Err(err) = bot
        .send_message(ChatId(telegram_id), i18n::with_footer(&lang, &body))
        .await
    {
        tracing::warn!(error=%err, "failed to send alarm notification");
    }
}

async fn notify_smart_alert(
    bot: &Bot,
    state: &AppState,
    telegram_id: i64,
    snapshot: &score_engine::IntelligenceSnapshot,
    specialized: Option<SpecializedSnapshot>,
    policy: AlertPolicy,
) {
    let lang = match state.user_store.get(telegram_id).await {
        Some(user) => user.language,
        None => "en".into(),
    };
    let reasons = if snapshot.explanation.reasons.is_empty() {
        "-".into()
    } else {
        snapshot.explanation.reasons.join(", ")
    };
    let warnings = if snapshot.explanation.warnings.is_empty() {
        "-".into()
    } else {
        snapshot.explanation.warnings.join(", ")
    };
    let weights = state.market_state.adaptive_weights(&snapshot.symbol);
    let extra = specialized
        .map(|specialized| {
            format!(
                "\nSweep: {:.1} {:?}\nShock: {:.1}\nDerivatives stress: {:.1}\nConflict: {:.1}\nRegime transition: {:.1}\nCross-market divergence: {:.1}",
                specialized.sweep.score,
                specialized.sweep.side,
                specialized.shock.score,
                specialized.derivatives_stress.score,
                specialized.conflict.score,
                specialized.regime_transition.score,
                specialized.cross_market.score
            )
        })
        .unwrap_or_default();
    let body = format!(
        "BOLDTRACE Intelligence — {}\nDecision: {:?}\nConfidence: {:.1}\nRisk: {:.1}\nScore: {:.1}\nData quality: {:.1}\nAdaptive weights V/F/OB/RSI: {:.2}/{:.2}/{:.2}/{:.2}\nPolicy: conf ≥ {:.1}, risk < {:.1}{}\nReasons: {}\nWarnings: {}",
        snapshot.symbol,
        snapshot.decision.decision,
        snapshot.decision.confidence,
        snapshot.decision.risk,
        snapshot.score,
        snapshot.data_quality,
        weights.volume_anomaly,
        weights.funding_extreme,
        weights.order_book_imbalance,
        weights.rsi_divergence,
        policy.min_confidence,
        policy.max_risk,
        extra,
        reasons,
        warnings
    );
    if let Err(err) = bot
        .send_message(ChatId(telegram_id), i18n::with_footer(&lang, &body))
        .await
    {
        tracing::warn!(error=%err, telegram_id, "failed to send smart intelligence alert");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neutral_specialized() -> SpecializedSnapshot {
        SpecializedSnapshot {
            sweep: score_engine::SweepResult {
                score: 0.0,
                side: SweepSide::None,
                rejection: 0.0,
            },
            shock: score_engine::ShockResult {
                score: 0.0,
                volume_shock: 0.0,
                range_shock: 0.0,
            },
            derivatives_stress: score_engine::DerivativesStress {
                score: 0.0,
                squeeze_risk: 0.0,
            },
            conflict: score_engine::ConflictResult {
                score: 0.0,
                force_no_trade: false,
            },
            regime_transition: score_engine::RegimeTransition {
                score: 0.0,
                transitioning: false,
            },
            cross_market: score_engine::CrossMarketResult {
                score: 0.0,
                divergence: 0.0,
            },
        }
    }

    #[test]
    fn conflicting_sweep_can_veto_direction() {
        let mut decision = score_engine::MetaDecision {
            decision: Decision::Long,
            signal_quality: 80.0,
            confidence: 80.0,
            risk: 20.0,
        };
        let mut specialized = neutral_specialized();
        specialized.sweep = score_engine::SweepResult {
            score: 90.0,
            side: SweepSide::Highs,
            rejection: 90.0,
        };
        let warnings = apply_specialized_gate(&mut decision, specialized);
        assert_eq!(decision.decision, Decision::NoTrade);
        assert!(warnings.iter().any(|warning| warning == "liquidity_sweep_veto"));
    }
}
