//! An export reads chapter files out of the data folder for its whole run, so it holds the
//! export lease (`.export.lease` in the data folder) shared until it ends, and a move takes it
//! exclusively: a move is refused while an export reads, and an export is refused while a move
//! runs, in this process or another. It is a lock file of its own, not the detector lease, so an
//! export can still take that one exclusively to download detector support.

use std::path::{Path, PathBuf};

use crate::domain::{Error, Result};
use crate::ml::lease::{self, Lease, Purpose};

pub const EXPORT_LEASE: &str = ".export.lease";
pub const EXPORT_LEASE_FILES: [&str; 2] = [EXPORT_LEASE, ".export.lease.owner"];
const TRIES: usize = 3;

/// The lease an export holds from before it looks up its chapter folders until it ends.
/// `data_dir` is read again once the lock is held: a move that finished in between switched the
/// folder, and the lock on the old one would guard nothing, so the new one is locked instead.
pub fn hold_for_export<E: From<Error>>(
    data_dir: impl Fn() -> std::result::Result<PathBuf, E>,
) -> std::result::Result<Lease, E> {
    for _ in 0..TRIES {
        let dir = data_dir()?;
        let lease = lease::share_at(&dir, EXPORT_LEASE)?;
        if data_dir()? == dir {
            return Ok(lease);
        }
    }
    Err(Error::Other("the data folder keeps moving; try again when it's done".into()).into())
}

/// Held by a move of `data_dir` from before it copies a chapter until the switch is done.
pub(super) fn hold_for_move(data_dir: &Path) -> Result<Lease> {
    lease::acquire_at(data_dir, EXPORT_LEASE, Purpose::Move)
}
