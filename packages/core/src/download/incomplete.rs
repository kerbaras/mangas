//! Errors for a chapter that did not fully download, worded for the user.

use crate::domain::Error;

/// Failures listed by number in the message; the rest are only counted.
const LISTED: usize = 3;

/// `failed` holds the 1-based index and error of every page that could not be fetched. A
/// browser verification wins over the count: the app turns it into a "Connect" prompt. A
/// refused address or a system without root certificates comes next: both are final, so the queue
/// must not retry the chapter.
pub fn incomplete(mut failed: Vec<(usize, Error)>, total: usize) -> Error {
    let challenge = |e: &Error| matches!(e, Error::Challenge { .. });
    let blocked = |e: &Error| matches!(e, Error::BlockedAddress { .. } | Error::NoRootCertificates);
    let first = |wanted: &dyn Fn(&Error) -> bool| failed.iter().position(|(_, e)| wanted(e));
    if let Some(at) = first(&challenge).or_else(|| first(&blocked)) {
        return failed.swap_remove(at).1;
    }
    failed.sort_by_key(|(index, _)| *index);
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
}
