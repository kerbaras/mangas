use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Token bucket: at most `capacity` requests in a burst, refilled at `per_second`.
pub struct TokenBucket {
    capacity: AtomicU32,
    per_second: AtomicU32,
    state: Mutex<BucketState>,
}

struct BucketState {
    tokens: f64,
    last: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32, per_second: u32) -> Self {
        Self {
            capacity: AtomicU32::new(capacity),
            per_second: AtomicU32::new(per_second),
            state: Mutex::new(BucketState {
                tokens: f64::from(capacity),
                last: Instant::now(),
            }),
        }
    }

    /// Lowers the burst and refill rate to at most these values; never raises them.
    pub fn tighten(&self, capacity: u32, per_second: u32) {
        self.capacity.fetch_min(capacity, Ordering::Relaxed);
        self.per_second
            .fetch_min(per_second.max(1), Ordering::Relaxed);
    }

    /// Current `(capacity, per_second)`.
    pub fn limits(&self) -> (u32, u32) {
        (
            self.capacity.load(Ordering::Relaxed),
            self.per_second.load(Ordering::Relaxed),
        )
    }

    /// Holds every caller back for `wait` (a host answered 429): the bucket goes into debt so
    /// the next token comes after `wait`. Overlapping pauses do not add up.
    pub async fn pause(&self, wait: Duration) {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let (capacity, per_second) = self.limits();
        let (capacity, per_second) = (f64::from(capacity), f64::from(per_second));
        let (tokens, _) = refill(
            state.tokens,
            now.duration_since(state.last),
            capacity,
            per_second,
        );
        state.last = now;
        state.tokens = tokens.min(-wait.as_secs_f64() * per_second);
    }

    /// Waits until a token is available and consumes it.
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut state = self.state.lock().await;
                let now = Instant::now();
                let (capacity, per_second) = self.limits();
                let (capacity, per_second) = (f64::from(capacity), f64::from(per_second));
                let elapsed = now.duration_since(state.last);
                let (tokens, wait) = refill(state.tokens, elapsed, capacity, per_second);
                state.last = now;
                state.tokens = tokens;
                if wait.is_zero() {
                    state.tokens -= 1.0;
                    return;
                }
                wait
            };
            tokio::time::sleep(wait).await;
        }
    }
}

/// Pure refill step: returns the new token count and how long to wait for the next token.
fn refill(tokens: f64, elapsed: Duration, capacity: f64, per_second: f64) -> (f64, Duration) {
    let tokens = (tokens + elapsed.as_secs_f64() * per_second).min(capacity);
    if tokens >= 1.0 {
        (tokens, Duration::ZERO)
    } else {
        (tokens, Duration::from_secs_f64((1.0 - tokens) / per_second))
    }
}
