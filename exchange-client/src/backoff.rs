//! Exponential backoff for WebSocket reconnect attempts.

use std::time::Duration;

/// Tracks the delay to wait before the next reconnect attempt, doubling on
/// every failure up to a fixed cap and resetting on success.
#[derive(Debug, Clone)]
pub struct ReconnectBackoff {
    initial: Duration,
    cap: Duration,
    current: Duration,
}

impl ReconnectBackoff {
    pub fn new(initial: Duration, cap: Duration) -> Self {
        Self {
            initial,
            cap,
            current: initial,
        }
    }

    /// Returns the delay to wait for the next attempt and advances the
    /// internal state for the following one.
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = std::cmp::min(self.current * 2, self.cap);
        delay
    }

    /// Resets the backoff after a successful connection.
    pub fn reset(&mut self) {
        self.current = self.initial;
    }
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_up_to_cap() {
        let mut backoff = ReconnectBackoff::new(Duration::from_secs(1), Duration::from_secs(8));
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));
        assert_eq!(backoff.next_delay(), Duration::from_secs(4));
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));
    }

    #[test]
    fn reset_returns_to_initial() {
        let mut backoff = ReconnectBackoff::new(Duration::from_secs(1), Duration::from_secs(8));
        backoff.next_delay();
        backoff.next_delay();
        backoff.reset();
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    }
}
