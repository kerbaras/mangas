use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Token bucket: at most `capacity` requests in a burst, refilled at `per_second`.
pub struct TokenBucket {
    capacity: f64,
    per_second: f64,
    state: Mutex<BucketState>,
}

struct BucketState {
    tokens: f64,
    last: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32, per_second: u32) -> Self {
        Self {
            capacity: f64::from(capacity),
            per_second: f64::from(per_second),
            state: Mutex::new(BucketState {
                tokens: f64::from(capacity),
                last: Instant::now(),
            }),
        }
    }

    /// Waits until a token is available and consumes it.
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut state = self.state.lock().await;
                let now = Instant::now();
                let (tokens, wait) = refill(
                    state.tokens,
                    now.duration_since(state.last),
                    self.capacity,
                    self.per_second,
                );
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
