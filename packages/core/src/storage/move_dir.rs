use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::copy::{check_empty, copy_all, discard_copy, plan, verify};
use super::layout::DATA_SUBDIRS;
use super::partial::{mark, prepare_target, unmark};
use super::reading::{hold_for_move, EXPORT_LEASE_FILES};
use super::relocate::{check_target, rebase};
use super::tree::{empty_dir, missing_dirs, remove_empty};
use crate::domain::{IoAt, Result};
use crate::library::{downloaded_chapters, get_settings, switch_data_dir, Library};
use crate::ml::lease::{self, Purpose};

/// Bytes copied so far out of the total to copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MoveProgress {
    pub copied: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Moved {
    pub data_dir: String,
    pub files: usize,
    pub bytes: u64,
    /// Old folders that couldn't be deleted after the switch; safe to delete by hand.
    pub leftovers: Vec<String>,
}

/// Moves the data folder: copies `chapters/` and `models/` from `from` to `to`, verifies every
/// file, then switches the chapter paths and `Settings.data_dir` in one transaction and only
/// then deletes the old copies. Any failure before the switch removes the partial copy and
/// leaves the library as it was; a killed move leaves it marked (see [`super::partial`]),
/// and a marked target is refused.
pub fn move_data_dir(
    lib: &Library,
    from: &Path,
    to: &Path,
    on_progress: &mut dyn FnMut(MoveProgress),
) -> Result<Moved> {
    move_data_dir_with(lib, from, to, false, on_progress)
}

/// [`move_data_dir`]; with `replace_partial`, a partial copy a killed move left in `to` is
/// deleted first (the files it copied; files added there by hand stay) and the move starts
/// over there. Folders created for `to` go again when the move fails or is refused.
pub fn move_data_dir_with(
    lib: &Library,
    from: &Path,
    to: &Path,
    replace_partial: bool,
    on_progress: &mut dyn FnMut(MoveProgress),
) -> Result<Moved> {
    // A download in any process (app or CLI) would keep writing into the old folder.
    use crate::download::queue::{hold_queue, MOVE_SETTLE};
    hold_queue(lib, MOVE_SETTLE, || {
        // Under the hold, so no other move creates `to` in between.
        let missing = missing_dirs(to);
        move_held(lib, from, to, replace_partial, on_progress)
            .inspect_err(|_| remove_empty(&missing))
    })
}

fn move_held(
    lib: &Library,
    from: &Path,
    to: &Path,
    replace_partial: bool,
    on_progress: &mut dyn FnMut(MoveProgress),
) -> Result<Moved> {
    fs::create_dir_all(to).at(to)?;
    let to = fs::canonicalize(to).at(to)?;
    let from_resolved = fs::canonicalize(from).unwrap_or_else(|_| from.to_path_buf());
    check_target(&from_resolved, &to)?;
    // Not under an export, nor a detector download or removal, in this process or another. The
    // export lease goes first: a move an export refuses never holds the detector lease the export
    // may need for its detectors.
    let reading = hold_for_move(&from_resolved)?;
    let lease = lease::acquire(&from_resolved.join("models"), Purpose::Move)?;
    let plan = plan(&from_resolved)?;
    let kept = prepare_target(lib, &from_resolved, &to, replace_partial, &plan)?;
    check_empty(&to, &kept)?;
    let total: u64 = plan.iter().flat_map(|(_, f)| f).map(|f| f.bytes).sum();
    let files = plan.iter().map(|(_, f)| f.len()).sum();
    let created: Vec<bool> = DATA_SUBDIRS.iter().map(|s| !to.join(s).exists()).collect();
    let mut marker = mark(lib, &to, &from_resolved)?;
    let switched = copy_all(&from_resolved, &to, &plan, total, &mut marker, on_progress)
        .and_then(|()| verify(&to, &plan, &kept))
        .and_then(|()| switch(lib, &[from, &from_resolved], &to));
    if switched.is_err() {
        discard_copy(&to, &plan, &created);
    }
    drop(marker);
    if let Err(e) = unmark(lib, &to) {
        log::warn!("clear the move marker: {e}");
    }
    switched?;
    // Empty the old models while still holding their lock, so a process with the old path can't
    // start a download into them; the lock files go with the folder after.
    if let Err(e) = empty_dir(&from_resolved.join("models"), &lease::LEASE_FILES) {
        log::warn!("empty the old models folder: {e}");
    }
    drop((lease, reading));
    for name in EXPORT_LEASE_FILES {
        let _ = fs::remove_file(from_resolved.join(name));
    }
    let leftovers = DATA_SUBDIRS
        .iter()
        .map(|sub| from_resolved.join(sub))
        .filter(|old| old.exists() && fs::remove_dir_all(old).is_err())
        .map(|old| old.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    // The old data folder itself goes too once nothing else is left in it (a symlink to it
    // stays, with its target, so the path never dangles).
    let _ = fs::remove_dir(from);
    Ok(Moved {
        data_dir: to.to_string_lossy().into_owned(),
        files,
        bytes: total,
        leftovers,
    })
}

/// Chapter paths are re-rooted against both the stored and the resolved old folder, since
/// either form may be on record; chapters stored elsewhere keep their path.
fn switch(lib: &Library, from: &[&Path], to: &Path) -> Result<()> {
    let moved: Vec<_> = downloaded_chapters(lib, None)?
        .into_iter()
        .filter_map(|c| {
            let path = PathBuf::from(&c.path);
            let new = from.iter().find_map(|f| rebase(&path, f, to))?;
            Some((c, new.to_string_lossy().into_owned()))
        })
        .collect();
    let mut settings = get_settings(lib)?;
    settings.data_dir = Some(to.to_string_lossy().into_owned());
    switch_data_dir(lib, &moved, &settings)
}
