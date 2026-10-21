//! Errors for a chapter that did not fully download, worded for the user.

use super::unresolved::is_unresolved;
use crate::domain::Error;

/// Failures listed by number in the message; the rest are only counted.
const LISTED: usize = 3;

/// `failed` holds the 1-based index and error of every page that could not be fetched. A
/// browser verification wins over the count: the app turns it into a "Connect" prompt. A
/// refused address, a refused certificate or a system without root certificates comes next: both are final, so the queue
/// must not retry the chapter. When every failed page is a host that can't be looked up, the
/// first one's error leads, so the queue fails the chapter at once ([`is_unresolved`]).
pub fn incomplete(mut failed: Vec<(usize, Error)>, total: usize) -> Error {
    let challenge = |e: &Error| matches!(e, Error::Challenge { .. });
    let blocked = |e: &Error| {
        matches!(
            e,
            Error::BlockedAddress { .. } | Error::NoRootCertificates | Error::Certificate { .. }
        )
    };
    let first = |wanted: &dyn Fn(&Error) -> bool| failed.iter().position(|(_, e)| wanted(e));
    if let Some(at) = first(&challenge).or_else(|| first(&blocked)) {
        return failed.swap_remove(at).1;
    }
    failed.sort_by_key(|(index, _)| *index);
    if let [(_, first), ..] = failed.as_slice() {
        if failed.iter().all(|(_, e)| is_unresolved(&e.to_string())) {
            let count = failed.len();
            return Error::Other(format!("{first} ({count} of {total} pages failed)"));
        }
    }
    let listed: Vec<String> = failed
        .iter()
        .take(LISTED)
        .map(|(index, e)| format!("page {index}: {e}"))
        .collect();
    let more = match failed.len().saturating_sub(LISTED) {
        0 => String::new(),
        n => format!("; {n} more"),
    };
    Error::Other(format!(
        "incomplete chapter: {} of {total} pages failed ({}{more})",
        failed.len(),
        listed.join("; ")
    ))
}

/// Every fetch succeeded, yet some page files are not in the chapter directory.
pub fn missing_on_disk(missing: usize, total: usize) -> Error {
    Error::Other(format!(
        "incomplete chapter: {missing} of {total} pages missing on disk"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_root_certificates_win_over_the_count() {
        let failed = vec![
            (1, Error::Other("timed out".into())),
            (2, Error::NoRootCertificates),
        ];
        assert!(matches!(incomplete(failed, 9), Error::NoRootCertificates));
    }

    #[test]
    fn unresolved_hosts_lead_and_the_queue_fails_the_chapter_at_once() {
        let failed = vec![
            (2, super::super::unresolved("b.test", "no such host")),
            (1, super::super::unresolved("a.test", "no such host")),
        ];
        let error = incomplete(failed, 9);
        let text = "network: couldn't resolve a.test: no such host (2 of 9 pages failed)";
        assert_eq!(error.to_string(), text);
        assert!(!crate::download::queue::classify(&error).retryable);
        let mixed = vec![
            (1, super::super::unresolved("a.test", "no such host")),
            (2, Error::Other("network: timed out".into())),
        ];
        let error = incomplete(mixed, 9);
        assert!(error.to_string().starts_with("incomplete chapter: 2 of 9"));
        assert!(crate::download::queue::classify(&error).retryable);
    }
}
