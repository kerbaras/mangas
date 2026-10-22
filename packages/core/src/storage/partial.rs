//! A data-folder move marks its target while it copies, like a chapter's `.part` pages: a move
//! killed midway leaves a marked partial copy. The next move to that folder may delete and reuse
//! it, and the storage status lists it, with a way to remove it, until then. The marker lists
//! the files the move started copying (see [`super::manifest`]); only those are ever deleted.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::layout::DATA_SUBDIRS;
use super::manifest::{copies, header, remove_copies, reuse, size};
use super::tree::{dir_size, Plan};
use crate::domain::{io_at, Error, Result};
use crate::download::queue::now_ms;
use crate::library::{pending_move, queue_gate, set_pending_move, take_queue_hold, Library};

/// The marker file in the root of a folder a move is copying into.
pub const MOVE_MARKER: &str = ".mangas-move-partial";

/// What an interrupted move left in a folder: the files it copied there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartialCopy {
    pub path: String,
    pub bytes: u64,
}

/// The partial copy in `dir`, when a move marked it.
pub fn partial_copy_at(dir: &Path) -> Option<PartialCopy> {
    let all = || {
        DATA_SUBDIRS
            .iter()
            .map(|sub| dir_size(&dir.join(sub)))
            .sum()
    };
    is_marked(dir).then(|| PartialCopy {
        path: dir.to_string_lossy().into_owned(),
        bytes: copies(dir).map_or_else(|_| all(), |c| size(dir, &c)),
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

/// Deletes the partial copy [`leftover_copy`] lists: the files the move copied and the marker;
/// anything else in that folder, files added by hand to its subfolders too, stays. Refused
/// while a move runs.
pub fn remove_leftover_copy(lib: &Library, data_dir: &Path) -> Result<Option<PartialCopy>> {
    if leftover_copy(lib, data_dir)?.is_none() {
        return Ok(None);
    }
    // Held briefly so a move can't start copying into the folder while it's emptied.
    let now = now_ms();
    if !take_queue_hold(lib, now, now + 60_000)? {
        return Err(Error::Other(
            "a data-folder move is in progress; try again when it's done".into(),
        ));
    }
    let wiped = wipe_recorded(lib, data_dir);
    crate::library::set_queue_held_until(lib, 0)?;
    wiped
}

/// Read again under the hold: a move that finished since the listing cleared its record.
fn wipe_recorded(lib: &Library, data_dir: &Path) -> Result<Option<PartialCopy>> {
    let Some(target) = pending_move(lib)? else {
        return Ok(None);
    };
    let target = Path::new(&target);
    let Some(copy) = partial_copy_at(target).filter(|_| !same_dir(target, data_dir)) else {
        return Ok(None);
    };
    wipe(target)?;
    set_pending_move(lib, None)?;
    Ok(Some(copy))
}

/// Readies `to` for a move from `from` copying `plan`: a partial copy there is deleted when
/// `replace` allows it, keeping the files added there by hand, which are returned. Refused while
/// another folder still holds a listed partial copy, so a new move never hides it from the
/// storage status.
pub(crate) fn prepare_target(
    lib: &Library,
    from: &Path,
    to: &Path,
    replace: bool,
    plan: &Plan,
) -> Result<BTreeSet<PathBuf>> {
    if let Some(other) = pending_move(lib)? {
        let other = Path::new(&other);
        if is_marked(other) && !same_dir(other, to) && !same_dir(other, from) {
            return Err(Error::Other(format!(
                "{} still holds a partial copy left by an interrupted move; remove it from the \
                 storage status first",
                other.display()
            )));
        }
    }
    if !is_marked(to) {
        return Ok(BTreeSet::new());
    }
    if !replace {
        return Err(Error::Other(format!(
            "{} holds a partial copy left by an interrupted move; move again replacing it \
             (--force deletes the files it copied there; files added by hand stay), or remove it \
             from the storage status",
            to.display()
        )));
    }
    wipe_for_reuse(to, plan)
}

pub(crate) fn is_marked(dir: &Path) -> bool {
    dir.join(MOVE_MARKER).is_file()
}

/// Marks `to` as a move from `from` before anything is copied into it; the copy appends each
/// file it starts to the returned marker.
pub(crate) fn mark(lib: &Library, to: &Path, from: &Path) -> Result<File> {
    let text = header(from)?;
    // Recorded first: a record without a marker is dropped by the next listing, while a marker
    // without a record would block the folder with nothing offering to remove it.
    set_pending_move(lib, Some(&to.to_string_lossy()))?;
    let path = to.join(MOVE_MARKER);
    let marked = fs::write(&path, text).and_then(|()| File::options().append(true).open(&path));
    marked.or_else(|e| {
        set_pending_move(lib, None)?;
        Err(io_at(&path)(e))
    })
}

/// Clears the mark once the copy is either the data folder or gone.
pub(crate) fn unmark(lib: &Library, to: &Path) -> Result<()> {
    remove_marker(to)?;
    set_pending_move(lib, None)
}

/// [`reuse`] of a marked folder, then its marker goes.
fn wipe_for_reuse(dir: &Path, plan: &Plan) -> Result<BTreeSet<PathBuf>> {
    let added = reuse(dir, plan)?;
    remove_marker(dir)?;
    Ok(added)
}

/// Deletes the files the partial copy in a marked folder is made of, then the marker.
pub(crate) fn wipe(dir: &Path) -> Result<()> {
    remove_copies(dir, &copies(dir)?)?;
    remove_marker(dir)
}

fn remove_marker(dir: &Path) -> Result<()> {
    match fs::remove_file(dir.join(MOVE_MARKER)) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(io_at(&dir.join(MOVE_MARKER))(e)),
        _ => Ok(()),
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
