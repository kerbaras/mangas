use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use super::shared::SharedState;

/// Token bucket: at most `capacity` requests in a burst, refilled at `per_second`. Once
/// [`TokenBucket::share_in`] gives it a file, its tokens live there and every process using
/// that file shares them; until then (or if the file cannot be used) they live in memory.
pub struct TokenBucket {
    capacity: AtomicU32,
    per_second: AtomicU32,
    state: Mutex<BucketState>,
    shared: OnceLock<SharedState>,
    /// Set once the fallback to in-memory tokens has been logged at warning level.
    warned: AtomicBool,
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
            shared: OnceLock::new(),
            warned: AtomicBool::new(false),
        }
    }

    /// Keeps the tokens in the file at `path` from now on (the first call wins).
    pub fn share_in(&self, path: PathBuf) {
        let _ = self.shared.set(SharedState::new(path));
    }

    /// Runs `op` on the shared file off the async threads; `None` when the bucket is not shared
    /// or the file failed, so the caller falls back to the in-memory tokens. A failure is a
    /// warning the first time per bucket (one per host per process), then a debug line.
    async fn on_file<T: Send + 'static>(
        &self,
        op: impl FnOnce(SharedState, f64, f64) -> std::io::Result<T> + Send + 'static,
    ) -> Option<T> {
        let shared = self.shared.get()?.clone();
        let path = shared.path().to_path_buf();
        let (capacity, per_second) = self.limits();
        let (capacity, per_second) = (f64::from(capacity), f64::from(per_second));
        let done = tokio::task::spawn_blocking(move || op(shared, capacity, per_second)).await;
        let failure = match done {
            Ok(Ok(value)) => return Some(value),
            Ok(Err(e)) => format!("{}: {e}", path.display()),
            Err(e) => format!("task failed: {e}"),
        };
        let level = match self.warned.swap(true, Ordering::Relaxed) {
            false => log::Level::Warn,
            true => log::Level::Debug,
        };
        log::log!(
            level,
            "shared rate limit unavailable, pacing in this process only ({failure})"
        );
        None
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
        let on_file = self.on_file(move |shared, c, r| shared.pause(wait, c, r));
        if on_file.await.is_some() {
            return;
        }
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
            let on_file = self.on_file(|shared, c, r| shared.take(c, r)).await;
            let wait = match on_file {
                Some(wait) => wait,
                None => self.take_in_memory().await,
            };
            if wait.is_zero() {
                return;
            }
            tokio::time::sleep(wait).await;
        }
    }

    /// Takes an in-memory token (zero wait) or says how long until the next one.
    async fn take_in_memory(&self) -> Duration {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let (capacity, per_second) = self.limits();
        let (capacity, per_second) = (f64::from(capacity), f64::from(per_second));
        let elapsed = now.duration_since(state.last);
        let (tokens, wait) = refill(state.tokens, elapsed, capacity, per_second);
        state.last = now;
        state.tokens = if wait.is_zero() { tokens - 1.0 } else { tokens };
        wait
    }
}

/// Pure refill step: returns the new token count and how long to wait for the next token.
pub(super) fn refill(
    tokens: f64,
    elapsed: Duration,
    capacity: f64,
    per_second: f64,
) -> (f64, Duration) {
    let tokens = (tokens + elapsed.as_secs_f64() * per_second).min(capacity);
    if tokens >= 1.0 {
        (tokens, Duration::ZERO)
    } else {
        (tokens, Duration::from_secs_f64((1.0 - tokens) / per_second))
    }
}
