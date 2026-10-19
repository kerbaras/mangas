//! Writing an export file safely: never a half-written one, and over an existing file only when
//! asked to (then atomically, so a failed export leaves the old file as it was).

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use crate::domain::{Error, Result};

/// Where an export path the user gave goes: an absolute path as given, a relative one (or none)
/// under `default_dir`, never under the process working directory.
pub fn in_export_folder(default_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        default_dir.join(path)
    }
}

/// Writes `path` with `fill` and returns the file written: over an existing file when `replace`
/// ([`write_over`]), else under a free name ([`write_new`]).
pub(crate) fn write_file(
    path: &Path,
    replace: bool,
    fill: impl FnOnce(File) -> Result<()>,
) -> Result<PathBuf> {
    match replace {
        true => write_over(path, fill),
        false => write_new(path, fill),
    }
}

/// Writes `path` (parent directories are created), replacing a file already there. The content
/// is built in a uniquely named `.part` file in the same folder and renamed over `path` once
/// complete, so readers see the old file or the new one, never a truncated one; on failure the
/// temporary file is removed and `path` is untouched.
fn write_over(path: &Path, fill: impl FnOnce(File) -> Result<()>) -> Result<PathBuf> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    let dir = dir.unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let mut builder = tempfile::Builder::new();
    builder.prefix(".mangas-").suffix(".part");
    // Same mode as `File::create` (0666 less the umask), not the temporary file's private 0600.
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o666));
    let tmp = builder.tempfile_in(dir)?;
    fill(tmp.as_file().try_clone()?)?;
    tmp.persist(path).map_err(|e| Error::from(e.error))?;
    Ok(path.to_path_buf())
}

/// Writes a new file at `path` (parent directories are created) with `fill` and returns the
/// file written. An existing file is never replaced: the name gets ` (2)`, ` (3)`, ... like a
/// browser download. The target is claimed with `create_new` (two exports racing for one name
/// get two files), the content is built in a `.part` file next to it and renamed over the claim
/// when complete, so the final name never holds a half-written file.
fn write_new(path: &Path, fill: impl FnOnce(File) -> Result<()>) -> Result<PathBuf> {
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

/// The file names in `dir`. A folder not created yet, or one we may write to but not list,
/// counts as empty: creating the file then decides, as it did before names were compared.
fn file_names(dir: &Path) -> std::io::Result<Vec<String>> {
    use std::io::ErrorKind::{NotFound, PermissionDenied};
    match std::fs::read_dir(dir) {
        Ok(entries) => Ok(entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()),
        Err(e) if matches!(e.kind(), NotFound | PermissionDenied) => Ok(Vec::new()),
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_folder_we_can_write_but_not_list_still_takes_the_export() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("drop");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o300)).unwrap();
        if std::fs::read_dir(&dir).is_ok() {
            return; // root ignores the mode; nothing to test
        }
        let wanted = vec!["Book.epub".to_string()];
        assert_eq!(free_names_in(&dir, &wanted).unwrap(), wanted);
        let written = write_file(&dir.join("Book.epub"), false, |_| Ok(()));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(written.unwrap(), dir.join("Book.epub"));
    }
}
