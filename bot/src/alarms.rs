//! Registered `/alarm` subscriptions and edge-triggered notification
//! logic: a user is notified the moment a symbol's score crosses their
//! threshold, not on every update while it stays above it.

use std::sync::Mutex;

struct Alarm {
    telegram_id: i64,
    symbol: String,
    threshold: f64,
    was_above: bool,
}

#[derive(Default)]
pub struct AlarmRegistry {
    alarms: Mutex<Vec<Alarm>>,
}

impl AlarmRegistry {
    /// Registers `telegram_id`'s alarm for `symbol` at `threshold`,
    /// replacing any existing alarm the user has for that symbol.
    pub fn set(&self, telegram_id: i64, symbol: &str, threshold: f64) {
        let mut alarms = self.alarms.lock().expect("alarm registry lock poisoned");
        alarms.retain(|a| !(a.telegram_id == telegram_id && a.symbol == symbol));
        alarms.push(Alarm {
            telegram_id,
            symbol: symbol.to_string(),
            threshold,
            was_above: false,
        });
    }

    /// Given a fresh `score` for `symbol`, returns the `(telegram_id,
    /// threshold)` pairs that should be notified now: alarms whose
    /// threshold the score has just crossed upward. Alarms already above
    /// threshold are not re-notified until they first drop back below it.
    pub fn crossed(&self, symbol: &str, score: f64) -> Vec<(i64, f64)> {
        let mut alarms = self.alarms.lock().expect("alarm registry lock poisoned");
        let mut triggered = Vec::new();
        for alarm in alarms.iter_mut().filter(|a| a.symbol == symbol) {
            let is_above = score >= alarm.threshold;
            if is_above && !alarm.was_above {
                triggered.push((alarm.telegram_id, alarm.threshold));
            }
            alarm.was_above = is_above;
        }
        triggered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_alarms_means_nothing_triggers() {
        let registry = AlarmRegistry::default();
        assert!(registry.crossed("BTCUSDT", 90.0).is_empty());
    }

    #[test]
    fn triggers_once_on_upward_crossing() {
        let registry = AlarmRegistry::default();
        registry.set(1, "BTCUSDT", 70.0);

        assert!(registry.crossed("BTCUSDT", 60.0).is_empty());
        assert_eq!(registry.crossed("BTCUSDT", 75.0), vec![(1, 70.0)]);
        // Stays above threshold: no repeat notification.
        assert!(registry.crossed("BTCUSDT", 80.0).is_empty());
    }

    #[test]
    fn re_triggers_after_dropping_back_below() {
        let registry = AlarmRegistry::default();
        registry.set(1, "BTCUSDT", 70.0);

        registry.crossed("BTCUSDT", 75.0);
        registry.crossed("BTCUSDT", 60.0);
        assert_eq!(registry.crossed("BTCUSDT", 80.0), vec![(1, 70.0)]);
    }

    #[test]
    fn replacing_an_alarm_resets_its_trigger_state() {
        let registry = AlarmRegistry::default();
        registry.set(1, "BTCUSDT", 70.0);
        registry.crossed("BTCUSDT", 75.0);

        registry.set(1, "BTCUSDT", 50.0);
        assert_eq!(registry.crossed("BTCUSDT", 60.0), vec![(1, 50.0)]);
    }
}
