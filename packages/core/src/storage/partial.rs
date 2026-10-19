//! A data-folder move marks its target while it copies, like a chapter's `.part` pages: a move
//! killed midway leaves a marked partial copy. The next move to that folder may wipe and reuse
//! it, and the storage status lists it, with a way to remove it, until then.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Serialize;

use super::layout::DATA_SUBDIRS;
use super::tree::dir_size;
use crate::domain::{Error, Result};
use crate::download::queue::now_ms;
use crate::library::{pending_move, queue_gate, set_pending_move, take_queue_hold, Library};

/// The marker file in the root of a folder a move is copying into.
pub const MOVE_MARKER: &str = ".mangas-move-partial";

/// What an interrupted move left in a folder: only the app's own subfolders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartialCopy {
    pub path: String,
    pub bytes: u64,
}

/// The partial copy in `dir`, when a move marked it.
pub fn partial_copy_at(dir: &Path) -> Option<PartialCopy> {
    is_marked(dir).then(|| PartialCopy {
        path: dir.to_string_lossy().into_owned(),
        bytes: DATA_SUBDIRS
            .iter()
            .map(|sub| dir_size(&dir.join(sub)))
            .sum(),
    })
}

/// The partial copy an interrupted move left behind, if its folder still holds one. Nothing
/// while a move runs (its copy isn't left behind yet). A record whose marker is gone (deleted
/// by hand), or that points at the data folder itself (killed after switching), is cleared.
pub fn leftover_copy(lib: &Library, data_dir: &Path) -> Result<Option<PartialCopy>> {
    let Some(target) = pending_move(lib)? else {
        return Ok(None);
    };
    if queue_gate(lib)?.held_until > now_ms() {
        return Ok(None);
    }
    let target = Path::new(&target);
    if same_dir(target, data_dir) {
        remove_marker(target)?;
        set_pending_move(lib, None)?;
        return Ok(None);
    }
    let copy = partial_copy_at(target);
    if copy.is_none() {
        set_pending_move(lib, None)?;
    }
    Ok(copy)
}

/// Deletes the partial copy [`leftover_copy`] lists: the app's subfolders in it and the marker;
/// anything else in that folder stays. Refused while a move runs.
pub fn remove_leftover_copy(lib: &Library, data_dir: &Path) -> Result<Option<PartialCopy>> {
    let Some(copy) = leftover_copy(lib, data_dir)? else {
        return Ok(None);
    };
    // Held briefly so a move can't start copying into the folder while it's emptied.
    let now = now_ms();
    if !take_queue_hold(lib, now, now + 60_000)? {
        return Err(Error::Other(
            "a data-folder move is in progress; try again when it's done".into(),
        ));
    }
    let wiped = wipe(Path::new(&copy.path)).and_then(|()| set_pending_move(lib, None));
    crate::library::set_queue_held_until(lib, 0)?;
    wiped.map(|()| Some(copy))
}

pub(crate) fn is_marked(dir: &Path) -> bool {
    dir.join(MOVE_MARKER).is_file()
}

/// Marks `to` as a move target before anything is copied into it.
pub(crate) fn mark(lib: &Library, to: &Path, from: &Path) -> Result<()> {
    fs::write(to.join(MOVE_MARKER), format!("{}\n", from.display()))?;
    set_pending_move(lib, Some(&to.to_string_lossy()))
}

/// Clears the mark once the copy is either the data folder or gone.
pub(crate) fn unmark(lib: &Library, to: &Path) -> Result<()> {
    remove_marker(to)?;
    set_pending_move(lib, None)
}

/// Removes the app's subfolders from a marked folder, then the marker.
pub(crate) fn wipe(dir: &Path) -> Result<()> {
    for sub in DATA_SUBDIRS {
        match fs::remove_dir_all(dir.join(sub)) {
            Err(e) if e.kind() != ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    remove_marker(dir)
}

fn remove_marker(dir: &Path) -> Result<()> {
    match fs::remove_file(dir.join(MOVE_MARKER)) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
