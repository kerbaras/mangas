//! Retry decisions as pure functions; the client only sleeps and loops.

use std::time::Duration;

use chrono::{DateTime, Utc};

/// Exponential backoff: `base_delay * 2^attempt`, capped at `max_delay`, at most `max_retries`
/// extra attempts. A server asking for a longer `Retry-After` than `max_delay` is not retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl RetryPolicy {
    pub const NONE: Self = Self {
        max_retries: 0,
        base_delay: Duration::ZERO,
        max_delay: Duration::ZERO,
    };

    pub fn backoff(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt);
        self.base_delay.saturating_mul(factor).min(self.max_delay)
    }

    /// Wait before retrying after failed attempt number `attempt` (0-based), or `None` to give
    /// up. `retry_after` is what the server asked for, if anything.
    pub fn delay(&self, attempt: u32, retry_after: Option<Duration>) -> Option<Duration> {
        if attempt >= self.max_retries {
            return None;
        }
        match retry_after {
            Some(wait) if wait > self.max_delay => None,
            Some(wait) => Some(wait),
            None => Some(self.backoff(attempt)),
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Statuses worth another try: rate limiting and server errors (except 501, which will not
/// change). Non-idempotent requests (POST) only retry when the server says it did not act.
pub fn retryable_status(status: u16, idempotent: bool) -> bool {
    match status {
        429 | 503 => true,
        501 => false,
        500..=599 => idempotent,
        _ => false,
    }
}

/// `Retry-After` as delay-seconds or an HTTP date (a date in the past means "now").
pub fn parse_retry_after(value: &str, now: DateTime<Utc>) -> Option<Duration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let at = DateTime::parse_from_rfc2822(value).ok()?;
    Some(
        at.with_timezone(&Utc)
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(Duration::ZERO),
    )
}
