//! Chapter-level retries, on top of the page client's own per-request retries.

use crate::domain::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Attempts per job before it is `Failed`, the first one included.
    pub max_attempts: u32,
    /// Wait before the second attempt; each later one waits four times longer.
    pub base_delay_ms: i64,
    pub max_delay_ms: i64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 15_000,
            max_delay_ms: 120_000,
        }
    }
}

/// Wait after the `attempts`-th failed attempt (1-based): 15 s, 60 s, then capped.
pub fn backoff(policy: &RetryPolicy, attempts: u32) -> i64 {
    let exponent = attempts.saturating_sub(1).min(16);
    policy
        .base_delay_ms
        .saturating_mul(4_i64.saturating_pow(exponent))
        .min(policy.max_delay_ms)
}

/// Why an attempt failed, worded for the user, and whether trying again can help.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    pub retryable: bool,
}

/// Errors that need the user (a browser check, a locked chapter, a full or read-only disk, no root
/// certificates on the system) or
/// that will not change (gone from the source: not found, HTTP 404/410) fail at once; network
/// trouble is retried.
pub fn classify(error: &Error) -> Failure {
    // Several failed pages come wrapped as "incomplete chapter: ... page 3: ... HTTP 404 ...".
    let gone = match error {
        Error::Source { message, .. } => is_gone(message),
        Error::Other(message) => message.starts_with("incomplete chapter") && is_gone(message),
        _ => false,
    };
    let retryable = !gone
        && !matches!(
            error,
            Error::Challenge { .. }
                | Error::BlockedAddress { .. }
                | Error::NoRootCertificates
                | Error::Unavailable { .. }
                | Error::NotFound(_)
                | Error::Io(_)
                | Error::Library(_)
                | Error::NotImplemented(_)
        );
    let message = match gone {
        true => format!("not found: {error}"),
        false => error.to_string(),
    };
    Failure { message, retryable }
}

/// Whether a failed job's stored error is one a retry cannot fix: the chapter is gone from the
/// source or its host is refused ([`Error::BlockedAddress`]). Mirrors the desktop's `isFinalError`.
pub fn is_final_error(error: &str) -> bool {
    error.contains(super::UNLISTED)
        || error.starts_with("not found")
        || (error.starts_with("refused:") && error.contains("resolves to a non-public address"))
}

/// The provider's "HTTP 404 for /path" style message for a resource that no longer exists.
fn is_gone(message: &str) -> bool {
    ["HTTP 404", "HTTP 410"]
        .iter()
        .any(|code| message.contains(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_root_certificates_fail_at_once() {
        let failure = classify(&Error::NoRootCertificates);
        assert!(!failure.retryable);
        assert!(
            failure.message.contains("ca-certificates"),
            "{}",
            failure.message
        );
    }
}
