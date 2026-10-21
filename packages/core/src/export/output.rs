//! Writing an export file safely: never a half-written one, and over an existing file only when
//! asked to (then atomically, so a failed export leaves the old file as it was).

use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};

use super::part_claim::{claim_free_name, claim_part, part_file, publish};
use super::space;
use crate::domain::{io_at, placed_at, IoAt, Result};
use crate::export::kindle::part_owner;

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
/// ([`write_over`]), else under a free name ([`write_new`]). A full disk is
/// [`Error::NoSpace`] in the file's folder.
pub(crate) fn write_file(
    path: &Path,
    replace: bool,
    fill: impl FnOnce(File) -> Result<()>,
) -> Result<PathBuf> {
    let written = match replace {
        true => write_over(path, fill),
        false => write_new(path, fill),
    };
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    written.map_err(|e| space::in_folder(e, dir.unwrap_or(Path::new("."))))
}

/// Writes `path` (parent directories are created), replacing a file already there. The content
/// is built in `<name>.part` in the same folder and renamed over `path` once complete, so readers
/// see the old file or the new one, never a truncated one; on failure the part is removed and
/// `path` is untouched. A part left by a killed export is taken over; when another replacement
/// of `path` holds the part, a uniquely named one is used.
fn write_over(path: &Path, fill: impl FnOnce(File) -> Result<()>) -> Result<PathBuf> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    let dir = dir.unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).at(dir)?;
    // Another replacement holds `<name>.part`, or the name is too long for a suffix.
    let file = match claim_part(path) {
        Ok(Some(file)) => file,
        Ok(None) => return write_over_unique(dir, path, fill),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidFilename => {
            return write_over_unique(dir, path, fill)
        }
        Err(e) => return Err(io_at(&part_file(path))(e)),
    };
    let part = part_file(path);
    let result = (file.try_clone().at(&part))
        .and_then(|file| fill(file).map_err(|e| placed_at(e, path)))
        .and_then(|()| std::fs::rename(&part, path).at(path));
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result.map(|()| path.to_path_buf())
}

/// [`write_over`] through a uniquely named `.part` in `dir`.
fn write_over_unique(
    dir: &Path,
    path: &Path,
    fill: impl FnOnce(File) -> Result<()>,
) -> Result<PathBuf> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(".mangas-").suffix(".part");
    // Same mode as `File::create` (0666 less the umask), not the temporary file's private 0600.
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o666));
    let tmp = builder.tempfile_in(dir).at(dir)?;
    fill(tmp.as_file().try_clone().at(tmp.path())?).map_err(|e| placed_at(e, path))?;
    tmp.persist(path).map_err(|e| io_at(path)(e.error))?;
    Ok(path.to_path_buf())
}

/// Writes a new file at `path` (parent directories are created) with `fill` and returns the
/// file written. An existing file is never replaced: the name gets ` (2)`, ` (3)`, ... like a
/// browser download. The content is built in `<name>.part`, held locked so two exports racing
/// for one name get two files, and renamed to the name once complete: the final name never
/// holds an empty or half-written file, even when the export is killed.
fn write_new(path: &Path, fill: impl FnOnce(File) -> Result<()>) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).at(parent)?;
    }
    let claim = claim_free_name(path)?;
    let result = (claim.file.try_clone().at(&claim.part))
        .and_then(|file| fill(file).map_err(|e| placed_at(e, &claim.target)))
        .and_then(|()| publish(&claim));
    if result.is_err() {
        let _ = std::fs::remove_file(&claim.part);
    }
    result.map(|()| claim.target)
}

/// The file names in `dir`. A folder not created yet, or one we may write to but not list,
/// counts as empty: creating the file then decides, as it did before names were compared.
pub(super) fn file_names(dir: &Path) -> std::io::Result<Vec<String>> {
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

pub(super) fn folded(names: &[String]) -> HashSet<String> {
    names.iter().map(|n| n.to_lowercase()).collect()
}

/// `name`, then `stem (2).ext`, `stem (3).ext`, ...: the order every export tries names in.
pub(super) fn candidate_names(name: &str) -> impl Iterator<Item = String> + '_ {
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
/// A book with parts in the folder (`Book (part 1).epub`) has its name taken too.
pub fn resolve_free_names(existing: &[String], wanted: &[String]) -> Vec<String> {
    let owners: Vec<String> = existing.iter().filter_map(|n| part_owner(n)).collect();
    let mut taken = folded(&[existing, &owners].concat());
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

/// [`resolve_free_names`] against the files in `dir` now. A folder we may write to but not list
/// is asked name by name instead ([`unlisted`](super::unlisted)), so its plan matches the files.
pub fn free_names_in(dir: &Path, wanted: &[String]) -> Result<Vec<String>> {
    match std::fs::read_dir(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            Ok(super::unlisted::free_names(dir, wanted))
        }
        _ => Ok(resolve_free_names(&file_names(dir).at(dir)?, wanted)),
    }
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
