//! A lease on the models folder shared by every mangas process (app and CLI), held as an OS
//! file lock on `models/.detectors.lease`. A download, a data-folder move and a removal take it
//! exclusively; loading the detectors for an export shares it. The OS releases the lock when
//! its process ends, so a crashed holder never leaves the folder locked.

use std::fs::{self, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use super::lease_fds::spawn as spawn_child;
use super::lease_fds::LeaseFile;
use crate::domain::{io_at, Error, IoAt, Result};

pub const LEASE_FILE: &str = ".detectors.lease";
/// Who holds the lease exclusively (a separate file: Windows can't read a locked one).
pub const OWNER_FILE: &str = ".detectors.lease.owner";
/// Files a move doesn't copy and clearing the folder doesn't delete.
pub const LEASE_FILES: [&str; 2] = [LEASE_FILE, OWNER_FILE];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Download,
    Move,
    Remove,
}

#[derive(Serialize, Deserialize)]
struct Owner {
    pid: u32,
    purpose: Purpose,
}

/// Held while the folder is in use; dropping it, or the process ending, releases it.
pub struct Lease {
    _file: LeaseFile,
    owner: Option<PathBuf>,
}

pub fn lease_path(models_dir: &Path) -> PathBuf {
    models_dir.join(LEASE_FILE)
}

/// Takes the lease to change the folder, or fails naming what holds it.
pub fn acquire(models_dir: &Path, purpose: Purpose) -> Result<Lease> {
    acquire_at(models_dir, LEASE_FILE, purpose)
}

/// [`acquire`] on the lock file `name` in `dir` (its owner record is `<name>.owner`).
pub fn acquire_at(dir: &Path, name: &str, purpose: Purpose) -> Result<Lease> {
    let file = open(dir, name)?;
    locked(file.file().try_lock(), dir, name)?;
    let owner = owner_path(dir, name);
    let record = Owner {
        pid: std::process::id(),
        purpose,
    };
    let bytes = serde_json::to_vec(&record).map_err(|e| Error::Other(e.to_string()))?;
    fs::write(&owner, bytes).at(&owner)?;
    Ok(Lease {
        _file: file,
        owner: Some(owner),
    })
}

/// Takes the lease to read the folder; any number of readers, refused during a change.
pub fn share(models_dir: &Path) -> Result<Lease> {
    share_at(models_dir, LEASE_FILE)
}

/// [`share`] on the lock file `name` in `dir`.
pub fn share_at(dir: &Path, name: &str) -> Result<Lease> {
    let file = open(dir, name)?;
    locked(file.file().try_lock_shared(), dir, name)?;
    // Nobody can hold it exclusively now, so an owner record left is from a crashed holder.
    if let Err(e) = remove_if_present(&owner_path(dir, name)) {
        log::warn!("clear a stale lease owner: {e}");
    }
    Ok(Lease {
        _file: file,
        owner: None,
    })
}

fn open(dir: &Path, name: &str) -> Result<LeaseFile> {
    fs::create_dir_all(dir).at(dir)?;
    let path = dir.join(name);
    let file = LeaseFile::open(|| {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
    })
    .at(&path)?;
    Ok(file)
}

fn locked(result: std::result::Result<(), TryLockError>, dir: &Path, name: &str) -> Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(TryLockError::WouldBlock) => Err(Error::Other(busy(dir, name))),
        Err(TryLockError::Error(e)) => Err(io_at(&dir.join(name))(e)),
    }
}

/// Why the lease is taken: the exclusive owner's purpose, else an export (reading chapters or
/// the detectors, see [`crate::storage::hold_for_export`]).
fn owner_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.owner"))
}

fn busy(dir: &Path, name: &str) -> String {
    let owner = fs::read(owner_path(dir, name))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Owner>(&bytes).ok());
    let Some(Owner { pid, purpose }) = owner else {
        return "an export or import is running; try again when it finishes".into();
    };
    let what = match purpose {
        Purpose::Download => "detector support is downloading",
        Purpose::Move => "the data folder is moving",
        Purpose::Remove => "detector support is being removed",
    };
    format!("{what} (mangas process {pid}); wait for it to finish or cancel it, then try again")
}

impl Drop for Lease {
    fn drop(&mut self) {
        // Before the lock goes (with the file), so a reader never sees a stale owner.
        let Some(owner) = &self.owner else { return };
        if let Err(e) = remove_if_present(owner) {
            log::warn!("release {}: {e}", owner.display());
        }
    }
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}
