//! Errors for a chapter that did not fully download, worded for the user.

use crate::domain::Error;

/// Failures listed by number in the message; the rest are only counted.
const LISTED: usize = 3;

/// `failed` holds the 1-based index and error of every page that could not be fetched. A
/// browser verification wins over the count: the app turns it into a "Connect" prompt.
pub fn incomplete(mut failed: Vec<(usize, Error)>, total: usize) -> Error {
    if let Some(at) = failed
        .iter()
        .position(|(_, e)| matches!(e, Error::Challenge { .. }))
    {
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
