//! Attaching the failing path to an IO error, so every [`Error::Io`] names the file or folder.

use std::path::{Path, PathBuf};

use super::{Error, Result};

/// The [`Error::Io`] for `source` on `path`.
pub fn io_at(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// `fs::copy`, its error placed on `src` when the source can't be read, else on `dst`.
pub(crate) fn copy_at(src: &Path, dst: &Path) -> Result<u64> {
    let failed = |e| {
        io_at(if std::fs::File::open(src).is_ok() {
            dst
        } else {
            src
        })(e)
    };
    std::fs::copy(src, dst).map_err(failed)
}

/// An IO error from a writer that only sees a `File`; [`placed_at`] names the file afterwards.
pub(crate) fn unplaced(source: std::io::Error) -> Error {
    Error::Io {
        path: PathBuf::new(),
        source,
    }
}

/// `e` naming `path` when it is an [`unplaced`] IO error, else unchanged.
pub(crate) fn placed_at(e: Error, path: &Path) -> Error {
    match e {
        Error::Io { path: p, source } if p.as_os_str().is_empty() => io_at(path)(source),
        e => e,
    }
}

/// `fs::write(&file, bytes).at(&file)?`: the IO result as a core result naming `path`.
pub trait IoAt<T> {
    fn at(self, path: impl AsRef<Path>) -> Result<T>;
}

impl<T> IoAt<T> for std::io::Result<T> {
    fn at(self, path: impl AsRef<Path>) -> Result<T> {
        self.map_err(io_at(path.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_names_the_path_once() {
        let e = std::fs::read(Path::new("/no/such/file"))
            .at(Path::new("/no/such/file"))
            .unwrap_err();
        let text = e.to_string();
        assert!(text.starts_with("io: /no/such/file: "), "{text}");
        assert_eq!(text.matches("/no/such/file").count(), 1, "{text}");
        let source = std::error::Error::source(&e).map(ToString::to_string);
        assert!(source.is_some_and(|s| text.ends_with(&s)), "{text}");
    }

    #[test]
    fn a_writer_error_is_placed_once() {
        let e = placed_at(
            unplaced(std::io::ErrorKind::Other.into()),
            Path::new("/a/b.epub"),
        );
        assert!(e.to_string().starts_with("io: /a/b.epub: "), "{e}");
        let again = placed_at(e, Path::new("/elsewhere"));
        assert!(again.to_string().starts_with("io: /a/b.epub: "), "{again}");
    }

    #[test]
    fn a_failed_copy_names_the_path_that_failed() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = manifest.join("Cargo.toml");
        let missing = manifest.join("no-such-dir-for-copy-at");
        let dst = missing.join("Cargo.toml");
        let path_of = |r: Result<u64>| match r {
            Err(Error::Io { path, .. }) => path,
            other => panic!("{other:?}"),
        };
        assert_eq!(path_of(copy_at(&dst, &src)), dst);
        assert_eq!(path_of(copy_at(&src, &dst)), dst);
        assert_eq!(
            path_of(copy_at(&missing.join("a"), &missing.join("b"))),
            missing.join("a")
        );
    }
}
