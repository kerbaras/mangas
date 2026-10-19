//! The TLS root store: recognising a system that has none (a minimal Linux install without the
//! `ca-certificates` package), so the error says what to install.

use std::error::Error as StdError;

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
}
