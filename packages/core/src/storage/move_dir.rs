use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::layout::DATA_SUBDIRS;
use super::partial::{mark, prepare_target, unmark};
use super::reading::{hold_for_move, EXPORT_LEASE_FILES};
use super::relocate::{check_target, rebase};
use super::tree::{empty_dir, list_files, FileEntry};
use crate::domain::{Error, Result};
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
/// deleted first and the move starts over there.
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
        move_held(lib, from, to, replace_partial, on_progress)
    })
}

fn move_held(
    lib: &Library,
    from: &Path,
    to: &Path,
    replace_partial: bool,
    on_progress: &mut dyn FnMut(MoveProgress),
) -> Result<Moved> {
    fs::create_dir_all(to)?;
    let to = fs::canonicalize(to)?;
    let from_resolved = fs::canonicalize(from).unwrap_or_else(|_| from.to_path_buf());
    check_target(&from_resolved, &to)?;
    // Not under a detector download or removal, nor an export, in this process or another.
    let lease = lease::acquire(&from_resolved.join("models"), Purpose::Move)?;
    let reading = hold_for_move(&from_resolved)?;
    prepare_target(lib, &from_resolved, &to, replace_partial)?;
    let mut plan = Vec::new();
    for sub in DATA_SUBDIRS {
        if !list_files(&to.join(sub))?.is_empty() {
            return Err(Error::Other(format!(
                "{} already has a {sub} folder with files in it; choose an empty folder",
                to.display()
            )));
        }
        let mut files = list_files(&from_resolved.join(sub))?;
        if sub == "models" {
            files.retain(|f| !lease::LEASE_FILES.iter().any(|l| f.rel == Path::new(l)));
        }
        plan.push((sub, files));
    }
    let total: u64 = plan.iter().flat_map(|(_, f)| f).map(|f| f.bytes).sum();
    let files = plan.iter().map(|(_, f)| f.len()).sum();
    let created: Vec<bool> = DATA_SUBDIRS.iter().map(|s| !to.join(s).exists()).collect();
    mark(lib, &to, &from_resolved)?;
    let switched = copy_all(&from_resolved, &to, &plan, total, on_progress)
        .and_then(|()| verify(&to, &plan))
        .and_then(|()| switch(lib, &[from, &from_resolved], &to));
    if switched.is_err() {
        discard_copy(&to, &plan, &created);
    }
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

type Plan<'a> = [(&'a str, Vec<FileEntry>)];

fn copy_all(
    from: &Path,
    to: &Path,
    plan: &Plan,
    total: u64,
    on_progress: &mut dyn FnMut(MoveProgress),
) -> Result<()> {
    let mut copied = 0;
    on_progress(MoveProgress { copied, total });
    for (sub, files) in plan {
        for file in files {
            let dst = to.join(sub).join(&file.rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(from.join(sub).join(&file.rel), &dst)?;
            copied += file.bytes;
            on_progress(MoveProgress { copied, total });
        }
    }
    Ok(())
}

fn verify(to: &Path, plan: &Plan) -> Result<()> {
    for (sub, files) in plan {
        if list_files(&to.join(sub))? != *files {
            return Err(Error::Other(format!(
                "the copied {sub} folder doesn't match the original; nothing was changed"
            )));
        }
    }
    Ok(())
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

/// Undoes a failed copy: folders the move created go entirely; in folders that were already
/// there (empty of files), only the copied files go.
fn discard_copy(to: &Path, plan: &Plan, created: &[bool]) {
    for ((sub, files), created) in plan.iter().zip(created) {
        if *created {
            fs::remove_dir_all(to.join(sub)).ok();
        } else {
            for file in files {
                fs::remove_file(to.join(sub).join(&file.rel)).ok();
            }
        }
    }
}
