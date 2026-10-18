//! Writing an export file safely: never over an existing file, never a half-written one.

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use crate::domain::{Error, Result};

/// Writes a new file at `path` (parent directories are created) with `fill` and returns the
/// file written. An existing file is never replaced: the name gets ` (2)`, ` (3)`, ... like a
/// browser download. The target is claimed with `create_new` (two exports racing for one name
/// get two files), the content is built in a `.part` file next to it and renamed over the claim
/// when complete, so the final name never holds a half-written file.
pub(crate) fn write_new(path: &Path, fill: impl FnOnce(File) -> Result<()>) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let (target, _claim) = claim_free_file(path)?;
    let tmp = part_file(&target);
    let result = File::create(&tmp)
        .map_err(Error::from)
        .and_then(fill)
        .and_then(|()| std::fs::rename(&tmp, &target).map_err(Error::from));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&target);
    }
    result.map(|()| target)
}

/// `<file name>.part` next to `target`; appended to the whole name so `book` and `book.epub`
/// never share one temporary file.
fn part_file(target: &Path) -> PathBuf {
    let mut name = target.file_name().map(OsString::from).unwrap_or_default();
    name.push(".part");
    target.with_file_name(name)
}

/// Atomically creates `path`, or the first `stem (n).ext` that does not exist yet.
fn claim_free_file(path: &Path) -> std::io::Result<(PathBuf, File)> {
    for candidate in candidates(path) {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    unreachable!("candidates is infinite")
}

fn candidates(path: &Path) -> impl Iterator<Item = PathBuf> + '_ {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    std::iter::once(path.to_path_buf())
        .chain((2..).map(move |n| path.with_file_name(format!("{stem} ({n}){ext}"))))
}

/// XML text/attribute escaping shared by the EPUB, ComicInfo and other XML writers.
pub(crate) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
