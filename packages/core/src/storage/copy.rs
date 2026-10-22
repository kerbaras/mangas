//! The copy steps of a data-folder move: what to copy, copying it, checking it, undoing it.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::layout::DATA_SUBDIRS;
use super::manifest::copy_line;
use super::move_dir::MoveProgress;
use super::partial::MOVE_MARKER;
use super::tree::{list_files, FileEntry, Plan};
use crate::domain::{copy_at, Error, IoAt, Result};
use crate::ml::lease;

/// The files to copy out of `from`'s `chapters/` and `models/` (not the models lease files).
pub(crate) fn plan(from: &Path) -> Result<Vec<(&'static str, Vec<FileEntry>)>> {
    let mut plan = Vec::new();
    for sub in DATA_SUBDIRS {
        let mut files = list_files(&from.join(sub))?;
        if sub == "models" {
            files.retain(|f| !lease::LEASE_FILES.iter().any(|l| f.rel == Path::new(l)));
        }
        plan.push((sub, files));
    }
    Ok(plan)
}

/// Refuses a target whose app subfolders hold files, other than the ones in `kept` (files
/// added by hand to a partial copy this move replaces).
pub(crate) fn check_empty(to: &Path, kept: &BTreeSet<PathBuf>) -> Result<()> {
    for sub in DATA_SUBDIRS {
        if !present(to, sub, kept)?.is_empty() {
            return Err(Error::Other(format!(
                "{} already has a {sub} folder with files in it; choose an empty folder",
                to.display()
            )));
        }
    }
    Ok(())
}

fn present(to: &Path, sub: &str, kept: &BTreeSet<PathBuf>) -> Result<Vec<FileEntry>> {
    let mut files = list_files(&to.join(sub))?;
    files.retain(|f| !kept.contains(&Path::new(sub).join(&f.rel)));
    Ok(files)
}

pub(crate) fn copy_all(
    from: &Path,
    to: &Path,
    plan: &Plan,
    total: u64,
    marker: &mut File,
    on_progress: &mut dyn FnMut(MoveProgress),
) -> Result<()> {
    let mut copied = 0;
    on_progress(MoveProgress { copied, total });
    for (sub, files) in plan {
        for file in files {
            let rel = Path::new(sub).join(&file.rel);
            let line = copy_line(&rel)?;
            marker.write_all(line.as_bytes()).at(to.join(MOVE_MARKER))?;
            let dst = to.join(&rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).at(parent)?;
            }
            copy_at(&from.join(sub).join(&file.rel), &dst)?;
            copied += file.bytes;
            on_progress(MoveProgress { copied, total });
        }
    }
    Ok(())
}

pub(crate) fn verify(to: &Path, plan: &Plan, kept: &BTreeSet<PathBuf>) -> Result<()> {
    for (sub, files) in plan {
        if present(to, sub, kept)? != *files {
            return Err(Error::Other(format!(
                "the copied {sub} folder doesn't match the original; nothing was changed"
            )));
        }
    }
    Ok(())
}

/// Undoes a failed copy: folders the move created go entirely; in folders that were already
/// there (empty of files, or holding only files added by hand), only the copied files go.
pub(crate) fn discard_copy(to: &Path, plan: &Plan, created: &[bool]) {
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
