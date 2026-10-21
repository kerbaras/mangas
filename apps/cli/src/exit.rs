//! Exit codes beyond anyhow's 1.

use std::fmt;

/// A failure that ends the process with its own exit `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exit {
    pub code: u8,
    pub message: String,
}

impl fmt::Display for Exit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Exit {}

/// The exit code for a failed command: an [`Exit`]'s own, 1 for anything else.
pub fn exit_code(error: &anyhow::Error) -> u8 {
    error.downcast_ref::<Exit>().map_or(1, |e| e.code)
}

/// `error` and its causes on one line, each reason once: a cause whose text its parent already
/// ends with (core's `io: {0}` over the io error itself) is not repeated.
pub fn describe(error: &anyhow::Error) -> String {
    let mut line = String::new();
    let mut parent = String::new();
    for cause in error.chain() {
        let text = cause.to_string();
        if !line.is_empty() && parent.ends_with(&text) {
            continue;
        }
        if !line.is_empty() {
            line.push_str(": ");
        }
        line.push_str(&text);
        parent = text;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::describe;
    use mangas_core::domain::Error;

    fn too_long() -> std::io::Error {
        std::io::Error::from_raw_os_error(36)
    }

    #[test]
    fn an_io_error_is_described_once() {
        let line = describe(&Error::from(too_long()).into());
        assert_eq!(line, format!("io: {}", too_long()));
    }

    #[test]
    fn context_and_distinct_causes_are_kept() {
        let e = anyhow::Error::from(Error::from(too_long())).context("writing b.epub");
        assert_eq!(describe(&e), format!("writing b.epub: io: {}", too_long()));
        let e = anyhow::anyhow!("inner").context("outer");
        assert_eq!(describe(&e), "outer: inner");
    }
}
