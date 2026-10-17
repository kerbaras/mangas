//! Reading one HTTP attempt: whether it may be retried and after how long.

use std::time::Duration;

use chrono::Utc;
use mangas_core::domain::Result;
use reqwest::Response;

use super::{parse_retry_after, retryable_status, Reply};

/// The outcome of one attempt and, when it is to be retried, how long to wait first.
pub type Attempt = (Result<Reply>, Option<Duration>);

pub fn mitigated(response: &Response) -> String {
    response
        .headers()
        .get("cf-mitigated")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

/// A challenge is never retried: only the user can pass it.
pub fn should_retry(response: &Response, idempotent: bool) -> bool {
    retryable_status(response.status().as_u16(), idempotent)
        && !mitigated(response).eq_ignore_ascii_case("challenge")
}

pub fn retry_after(response: &Response) -> Option<Duration> {
    let value = response.headers().get("retry-after")?.to_str().ok()?;
    parse_retry_after(value, Utc::now())
}

/// A failed connection never reached the server; a timeout or a dropped connection might have,
/// so only idempotent requests retry those.
pub fn retryable_transport(e: &reqwest::Error, idempotent: bool) -> bool {
    e.is_connect() || (idempotent && (e.is_timeout() || e.is_request()))
}
