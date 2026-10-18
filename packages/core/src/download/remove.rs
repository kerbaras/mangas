//! Removing files from a chapter folder by name.

use std::io;
use std::path::{Path, PathBuf};

use super::complete::is_marker_leftover;
use crate::domain::Result;

/// Removes the files in `dest` whose name matches `drop`, stopping at the first failure.
pub(super) fn remove_files(dest: &Path, drop: impl Fn(&str) -> bool) -> Result<()> {
    for path in matching_files(dest, drop)? {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Drops every `.part` file and stray aside marker file left in a completed chapter's folder.
/// One that can't be removed only gets a warning (the pages are all there) and the others go.
pub(super) fn drop_stale_parts(dest: &Path) {
    drop_stale_parts_with(dest, |path| std::fs::remove_file(path));
}

fn drop_stale_parts_with(dest: &Path, remove: impl Fn(&Path) -> io::Result<()>) {
    let stale = match matching_files(dest, |n| n.ends_with(".part") || is_marker_leftover(n)) {
        Ok(stale) => stale,
        Err(e) => return log::warn!("could not list .part files in {}: {e}", dest.display()),
    };
    for (path, e) in remove_each(stale, remove) {
        log::warn!("could not remove stale {}: {e}", path.display());
    }
}

/// Tries every path; returns the ones that could not be removed, with why.
fn remove_each(
    paths: Vec<PathBuf>,
    remove: impl Fn(&Path) -> io::Result<()>,
) -> Vec<(PathBuf, io::Error)> {
    paths
        .into_iter()
        .filter_map(|path| remove(&path).err().map(|e| (path, e)))
        .collect()
}

/// The files in `dest` whose name matches `drop`, in name order.
/// A name that is not valid UTF-8 is matched on its lossy spelling but kept by its real one.
fn matching_files(dest: &Path, drop: impl Fn(&str) -> bool) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dest)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && drop(&entry.file_name().to_string_lossy()) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_file_that_cant_be_removed_does_not_keep_the_next_one() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.part", "b.part", "0001.png"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        let stuck = dir.path().join("a.part");
        drop_stale_parts_with(dir.path(), |path| match path == stuck {
            true => Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            false => std::fs::remove_file(path),
        });
        assert!(stuck.exists(), "the stuck file is still there");
        assert!(!dir.path().join("b.part").exists(), "the next stray went");
        assert!(dir.path().join("0001.png").exists(), "pages stay");
    }

    #[test]
    fn every_failure_is_reported() {
        let paths = vec![PathBuf::from("a.part"), PathBuf::from("b.part")];
        let failed = remove_each(paths, |_| {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        });
        let failed: Vec<PathBuf> = failed.into_iter().map(|(path, _)| path).collect();
        assert_eq!(failed, [PathBuf::from("a.part"), PathBuf::from("b.part")]);
    }
}
