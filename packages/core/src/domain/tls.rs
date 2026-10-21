//! The TLS root store: recognising a system that has none (a minimal Linux install without the
//! `ca-certificates` package), so the error says what to install.

use std::error::Error as StdError;

use super::CertProblem;

/// What the user sees when no TLS client can be built for lack of root certificates.
pub const NO_ROOT_CERTIFICATES: &str = "no CA certificates found on this system, so secure \
     connections can't be checked; install the ca-certificates package (e.g. `sudo apt install \
     ca-certificates`)";

/// The verifier's message when the system store is empty (`rustls-platform-verifier`).
const VERIFIER_MESSAGE: &str = "No CA certificates were loaded from the system";

/// Whether `err`, or any error behind it, is the verifier finding no root certificates.
pub fn missing_root_certificates(err: &(dyn StdError + 'static)) -> bool {
    let mut next = Some(err);
    while let Some(e) = next {
        if e.to_string().contains(VERIFIER_MESSAGE) {
            return true;
        }
        next = e.source();
    }
    false
}

/// Whether the system store holds any root certificate (the one a webview checks too).
pub fn system_has_root_certificates() -> bool {
    !rustls_native_certs::load_native_certs().certs.is_empty()
}

/// Why a page failed TLS in a webview (the Connect window): the missing store when the system
/// has no roots, else the certificate problem WebKit reported for `host`.
pub fn webview_tls_failure(host: &str, problem: &CertProblem, has_roots: bool) -> String {
    if has_roots {
        super::Error::Certificate {
            host: host.to_string(),
            problem: problem.clone(),
        }
        .to_string()
    } else {
        NO_ROOT_CERTIFICATES.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("builder error")]
    struct Outer(#[source] std::io::Error);

    #[test]
    fn finds_the_verifier_message_anywhere_in_the_chain() {
        let inner = std::io::Error::other(format!("unexpected error: {VERIFIER_MESSAGE}"));
        assert!(missing_root_certificates(&Outer(inner)));
        let other = Outer(std::io::Error::other("connection refused"));
        assert!(!missing_root_certificates(&other));
    }

    #[test]
    fn webview_tls_failure_names_the_empty_store_or_the_certificate() {
        assert_eq!(
            webview_tls_failure("comick.io", &CertProblem::UnknownIssuer, false),
            NO_ROOT_CERTIFICATES
        );
        let bad = webview_tls_failure("comick.io", &CertProblem::Expired, true);
        assert_eq!(
            bad,
            "network: the certificate of comick.io couldn't be verified (expired)"
        );
    }
}
