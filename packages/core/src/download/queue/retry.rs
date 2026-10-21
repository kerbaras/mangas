//! Chapter-level retries, on top of the page client's own per-request retries.

use crate::domain::Error;
use crate::download::is_unresolved;

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
/// trouble is retried, except a page host that can't be looked up ([`is_unresolved`]): it fails at
/// once too, still retryable by hand.
pub fn classify(error: &Error) -> Failure {
    // Several failed pages come wrapped as "incomplete chapter: ... page 3: ... HTTP 404 ...".
    let gone = match error {
        Error::Source { message, .. } => is_gone(message),
        Error::Other(message) => message.starts_with("incomplete chapter") && is_gone(message),
        _ => false,
    };
    let unresolved = matches!(error, Error::Other(message) if is_unresolved(message));
    let retryable = !gone
        && !unresolved
        && !matches!(
            error,
            Error::Challenge { .. }
                | Error::BlockedAddress { .. }
                | Error::NoRootCertificates
                | Error::Certificate { .. }
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
/// source, or its host is refused ([`Error::BlockedAddress`]) while no proxy is in use. Through a
/// proxy the proxy resolves page hosts (#305), so a refusal stored before it was set may download
/// on a retry (#309). A host the URL names as private (an IP literal, `localhost`) is refused
/// before any proxy, so it stays final. Mirrors the desktop's `isFinalError`.
pub fn is_final_error(error: &str, proxied: bool) -> bool {
    error.contains(super::UNLISTED)
        || error.starts_with("not found")
        || (is_refused(error) && (!proxied || refused_literal(error)))
        || is_certificate_error(error)
}

/// Whether a refusal names an IP literal or a `localhost` host rather than a DNS name.
fn refused_literal(error: &str) -> bool {
    let rest = error.trim_start_matches("refused:").trim_start();
    let host = rest.split(" resolves").next().unwrap_or_default();
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.parse::<std::net::IpAddr>().is_ok() || host == "localhost" || host.ends_with(".localhost")
}

/// [`Error::BlockedAddress`]'s message.
pub fn is_refused(error: &str) -> bool {
    error.starts_with("refused:") && error.contains("resolves to a non-public address")
}

/// A stored [`Error::Certificate`] message, also when it is a page inside "incomplete chapter".
fn is_certificate_error(error: &str) -> bool {
    error.contains("network: the certificate of ") && error.contains(" couldn't be verified (")
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
    fn through_a_proxy_only_a_named_host_s_refusal_may_be_retried() {
        let named = "refused: cdn.example resolves to a non-public address 10.0.0.1";
        assert!(is_final_error(named, false));
        assert!(!is_final_error(named, true));
        for literal in [
            "refused: 10.0.0.5 resolves to a non-public address 10.0.0.5",
            "refused: [::1] resolves to a non-public address ::1",
            "refused: cdn.localhost resolves to a non-public address 127.0.0.1",
        ] {
            assert!(is_final_error(literal, true), "{literal}");
        }
    }

    #[test]
    fn a_refused_certificate_fails_at_once_and_stays_final() {
        let error = Error::Certificate {
            host: "comick.live".into(),
            problem: crate::domain::CertProblem::Expired,
        };
        let failure = classify(&error);
        assert!(!failure.retryable);
        assert!(
            is_final_error(&failure.message, false),
            "{}",
            failure.message
        );
        assert!(!is_final_error("network: error sending request", false));
    }

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
