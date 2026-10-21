//! Claiming a free export name through its `.part` file (#301): the final name only ever
//! appears by renaming a finished part, so an interrupted export leaves at most `<name>.part`,
//! never an empty or half-written book.

use std::ffi::OsString;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::output::{candidate_names, file_names, folded};
use crate::domain::{io_at, IoAt, Result};

/// A free name and its part, held locked for the whole write.
pub(super) struct Claim {
    pub target: PathBuf,
    pub part: PathBuf,
    pub file: File,
}

/// `<file name>.part` next to `target`; appended to the whole name so `book` and `book.epub`
/// never share one temporary file.
pub(super) fn part_file(target: &Path) -> PathBuf {
    let mut name = target.file_name().map(OsString::from).unwrap_or_default();
    name.push(".part");
    target.with_file_name(name)
}

/// The first of `path`, `stem (2).ext`, ... with no file and no part another export is writing
/// (it holds the part locked). A part left by an interrupted export is taken over, emptied.
/// Names already in the folder in another case count as taken, so case-insensitive disks match
/// the preview.
pub(super) fn claim_free_name(path: &Path) -> Result<Claim> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    let folder = dir.unwrap_or(Path::new("."));
    let taken = folded(&file_names(folder).at(folder)?);
    let free = candidate_names(&name).filter(|c| !taken.contains(&c.to_lowercase()));
    for target in free.map(|n| path.with_file_name(n)) {
        if exists(&target).at(&target)? {
            continue;
        }
        let Some(file) = claim_part(&target).at(part_file(&target))? else {
            continue;
        };
        if exists(&target).at(&target)? {
            continue;
        }
        let part = part_file(&target);
        return Ok(Claim { target, part, file });
    }
    unreachable!("candidate_names is infinite")
}

/// `target`'s `.part`, new and locked, or `None` when another export holds it. A part left by an
/// interrupted export is removed and created again: never emptied in place, since its file may be
/// the published book too (a kill between [`publish`]'s link and unlink).
pub(super) fn claim_part(target: &Path) -> std::io::Result<Option<File>> {
    let part = part_file(target);
    if let Some(file) = create_part(&part)? {
        return Ok(Some(file));
    }
    let Some(held) = stale(&part)? else {
        return Ok(None);
    };
    match std::fs::remove_file(&part) {
        Err(e) if e.kind() != ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    // Closed first: Windows keeps a removed name until its last handle closes. Another process
    // still holding it open leaves the name taken; the next name is free then.
    drop(held);
    match create_part(&part) {
        Err(e) if e.kind() == ErrorKind::PermissionDenied => Ok(None),
        claimed => claimed,
    }
}

/// A new `part`, locked; `None` when the name is there or another export locked it first. Without
/// locks (some network shares) the exclusive create alone decides.
fn create_part(part: &Path) -> std::io::Result<Option<File>> {
    match OpenOptions::new().write(true).create_new(true).open(part) {
        Ok(file) => Ok((lock(&file) != Lock::Held).then_some(file)),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(None),
        Err(e) => Err(e),
    }
}

/// `part` locked, when it is a leftover: nobody holds its lock and it is still the file at that
/// name (not one an export published or replaced while we opened it). Without locks a live part
/// can't be told from a leftover, so it counts as live.
fn stale(part: &Path) -> std::io::Result<Option<File>> {
    let file = match OpenOptions::new().write(true).open(part) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if lock(&file) != Lock::Ours {
        return Ok(None);
    }
    let at_name = match same_file::Handle::from_path(part) {
        Ok(handle) => handle,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let ours = same_file::Handle::from_file(file.try_clone()?)?;
    Ok((ours == at_name).then_some(file))
}

/// Publishes the finished part under its name, never over a file that appeared there meanwhile:
/// a hard link fails on an existing name. Disks without hard links (FAT) check, then rename.
/// A failure names the book's name, or the part when only removing the linked part failed.
pub(super) fn publish(claim: &Claim) -> Result<()> {
    let at_target = io_at(&claim.target);
    let appeared = || std::io::Error::new(ErrorKind::AlreadyExists, "appeared while exporting");
    match std::fs::hard_link(&claim.part, &claim.target) {
        Ok(()) => std::fs::remove_file(&claim.part).at(&claim.part),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Err(at_target(appeared())),
        Err(_) if exists(&claim.target).at(&claim.target)? => Err(at_target(appeared())),
        Err(_) => std::fs::rename(&claim.part, &claim.target).map_err(at_target),
    }
}

#[derive(PartialEq, Eq)]
enum Lock {
    Ours,
    Held,
    Unsupported,
}

/// Tries the part's lock: `Held` when another export holds it. Any other failure means the disk
/// has no locks (`ENOLCK` on NFS without lockd, unsupported on some SMB shares).
fn lock(file: &File) -> Lock {
    match file.try_lock() {
        Ok(()) => Lock::Ours,
        Err(TryLockError::WouldBlock) => Lock::Held,
        Err(TryLockError::Error(e)) => {
            tracing::debug!("no lock on the export part: {e}");
            Lock::Unsupported
        }
    }
}

fn exists(path: &Path) -> std::io::Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
#[path = "part_claim_tests.rs"]
mod tests;
