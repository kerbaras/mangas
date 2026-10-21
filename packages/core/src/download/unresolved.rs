//! A page host that can't be looked up, directly or by the proxy (#311). Worded here so the
//! page client, the incomplete-chapter error and the queue agree on it: the page client retries
//! it once, and the queue fails the chapter at once (a name that doesn't resolve now won't in a
//! minute) while leaving it retryable by hand.

use crate::domain::Error;

const UNRESOLVED: &str = "network: couldn't resolve ";
const PROXY_UNRESOLVED: &str = "; it may not resolve that name";

/// The local lookup of `host` failed with `cause`.
pub fn unresolved(host: &str, cause: &str) -> Error {
    Error::Other(format!("{UNRESOLVED}{host}: {cause}"))
}

/// `proxy` (worded for the user) answered `what` instead of reaching `host`.
pub fn unreached_through(proxy: &str, host: &str, what: &str) -> Error {
    Error::Other(format!(
        "network: {proxy} couldn't reach {host} ({what}){PROXY_UNRESOLVED}"
    ))
}

/// Whether `message` is (or starts with) one of the errors above.
pub fn is_unresolved(message: &str) -> bool {
    message.starts_with(UNRESOLVED)
        || (message.starts_with("network: ") && message.contains(PROXY_UNRESOLVED))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_wordings_are_recognised_and_nothing_else() {
        assert!(is_unresolved(
            &unresolved("a.test", "no such host").to_string()
        ));
        let proxied = unreached_through("the proxy http://p:8888", "a.test", "HTTP 500");
        assert!(is_unresolved(&proxied.to_string()));
        assert!(!is_unresolved(
            "network: error sending request for url (https://a.test/)"
        ));
        assert!(!is_unresolved(
            "incomplete chapter: 1 of 2 pages failed (page 1: timed out)"
        ));
    }
}
