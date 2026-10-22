//! Stdout writes that report errors instead of panicking like `println!`, so piping into
//! `head` ends the command quietly once the reader goes away.

use std::io::ErrorKind;

/// `println!` that returns the `io::Result` of the write.
macro_rules! outln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        writeln!(std::io::stdout().lock(), $($arg)*)
    }};
}

/// `print!` that returns the `io::Result` of the write and flushes.
macro_rules! out {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let mut stdout = std::io::stdout().lock();
        write!(stdout, $($arg)*).and_then(|()| stdout.flush())
    }};
}

pub(crate) use {out, outln};

/// `value` as pretty-printed JSON on its own line: the one serialiser of every `--json` output
/// that shares a shape (a chapter in `update --json` is the same object as in `chapters --json`).
pub fn print_json<T: serde::Serialize + ?Sized>(value: &T) -> anyhow::Result<()> {
    outln!("{}", serde_json::to_string_pretty(value)?)?;
    Ok(())
}

/// Treats a closed stdout as success: the reader (`head`, `grep -m1`) has what it wanted.
/// Only a top-level `io::Error` counts, so a broken connection inside a provider still fails.
pub fn ignore_broken_pipe(result: anyhow::Result<()>) -> anyhow::Result<()> {
    match result {
        Err(e) if is_broken_pipe(&e) => Ok(()),
        other => other,
    }
}

fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.downcast_ref::<std::io::Error>()
        .is_some_and(|io| io.kind() == ErrorKind::BrokenPipe)
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn broken_pipe_is_success() {
        let err = io::Error::from(ErrorKind::BrokenPipe);
        assert!(ignore_broken_pipe(Err(err.into())).is_ok());
    }

    #[test]
    fn other_errors_still_fail() {
        let err = io::Error::from(ErrorKind::PermissionDenied);
        assert!(ignore_broken_pipe(Err(err.into())).is_err());
        assert!(ignore_broken_pipe(Err(anyhow::anyhow!("boom"))).is_err());
    }

    #[test]
    fn nested_broken_pipe_still_fails() {
        let err = mangas_core::Error::Io {
            path: "/out".into(),
            source: io::Error::from(ErrorKind::BrokenPipe),
        };
        assert!(ignore_broken_pipe(Err(err.into())).is_err());
    }
}
