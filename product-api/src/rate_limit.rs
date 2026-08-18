//! In-memory sliding-window rate limiting for auth endpoints, keyed by the
//! normalized email an attempt was made against. This blocks
//! credential-stuffing against one account regardless of how many source
//! IPs it comes from (which, behind a platform load balancer, are not
//! reliably attributable without trusting a forwarded-for header). An
//! attacker rotating emails is not slowed by this alone, but combined with
//! Argon2's cost and the account-existence-blind login response, this
//! covers the realistic single-account brute-force case.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct RateLimiter {
    attempts: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RateLimiter {
    /// Records an attempt for `key` and returns `true` if it is allowed
    /// (fewer than `max` attempts recorded within `window`), `false` if the
    /// caller should be rejected with 429.
    pub fn check(&self, key: &str, max: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut attempts = self.attempts.lock().expect("rate limiter lock poisoned");
        let entry = attempts.entry(key.to_string()).or_default();
        while entry.front().is_some_and(|t| now.duration_since(*t) > window) {
            entry.pop_front();
        }
        if entry.len() >= max {
            return false;
        }
        entry.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_max_attempts_then_blocks() {
        let limiter = RateLimiter::default();
        let window = Duration::from_secs(60);
        for _ in 0..3 {
            assert!(limiter.check("a@b.com", 3, window));
        }
        assert!(!limiter.check("a@b.com", 3, window));
    }

    #[test]
    fn keys_are_independent() {
        let limiter = RateLimiter::default();
        let window = Duration::from_secs(60);
        assert!(limiter.check("a@b.com", 1, window));
        assert!(limiter.check("c@d.com", 1, window));
        assert!(!limiter.check("a@b.com", 1, window));
    }
}
