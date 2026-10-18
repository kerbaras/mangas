//! Writing an export file safely: never over an existing file, never a half-written one.

use std::collections::HashSet;
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

/// Atomically creates `path`, or the first `stem (n).ext` that is free. Names already in the
/// folder in another case count as taken, so case-insensitive disks match the preview.
fn claim_free_file(path: &Path) -> std::io::Result<(PathBuf, File)> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    let taken = folded(&file_names(dir.unwrap_or(Path::new(".")))?);
    let free = candidate_names(&name).filter(|c| !taken.contains(&c.to_lowercase()));
    for candidate in free.map(|n| path.with_file_name(n)) {
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
    unreachable!("candidate_names is infinite")
}

/// The file names in `dir`; a folder not created yet is empty.
fn file_names(dir: &Path) -> std::io::Result<Vec<String>> {
    match std::fs::read_dir(dir) {
        Ok(entries) => Ok(entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

fn folded(names: &[String]) -> HashSet<String> {
    names.iter().map(|n| n.to_lowercase()).collect()
}

/// `name`, then `stem (2).ext`, `stem (3).ext`, ...: the order every export tries names in.
fn candidate_names(name: &str) -> impl Iterator<Item = String> + '_ {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    std::iter::once(name.to_owned()).chain((2..).map(move |n| format!("{stem} ({n}){ext}")))
}

/// The names `wanted` get when written, in order, into a folder that holds `existing`: each
/// takes its first free candidate (ignoring case), as [`write_new`] does, so preview and files agree.
pub fn resolve_free_names(existing: &[String], wanted: &[String]) -> Vec<String> {
    let mut taken = folded(existing);
    wanted
        .iter()
        .map(|name| {
            let free = candidate_names(name)
                .find(|c| !taken.contains(&c.to_lowercase()))
                .unwrap_or_else(|| name.clone());
            taken.insert(free.to_lowercase());
            free
        })
        .collect()
}

/// [`resolve_free_names`] against the files in `dir` now.
pub fn free_names_in(dir: &Path, wanted: &[String]) -> Result<Vec<String>> {
    Ok(resolve_free_names(&file_names(dir)?, wanted))
}

/// XML text/attribute escaping shared by the EPUB, ComicInfo and other XML writers.
pub(crate) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
