//! The files a move copied, listed in its marker: a JSON header line, then one JSON string per
//! file, appended just before that file's copy starts. Reusing or removing the partial copy a
//! killed move left deletes only those, so files added there by hand stay, even where the move
//! would have copied one later. A marker written before the list existed only names the source
//! folder; the files that folder holds stand in for the list then.

use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::layout::DATA_SUBDIRS;
use super::partial::MOVE_MARKER;
use super::tree::{list_files, Plan};
use crate::domain::{Error, Result};

#[derive(Serialize, Deserialize)]
struct Header {
    from: String,
}

/// The paths (relative to the target) a move copying `plan` writes.
pub(crate) fn planned(plan: &Plan) -> BTreeSet<PathBuf> {
    plan.iter()
        .flat_map(|(sub, files)| files.iter().map(move |f| Path::new(sub).join(&f.rel)))
        .collect()
}

/// The first line of the marker of a move from `from`.
pub(crate) fn header(from: &Path) -> Result<String> {
    let header = Header {
        from: from.to_string_lossy().into_owned(),
    };
    Ok(json(&header)? + "\n")
}

/// The marker line for a copy of `rel` (relative to the target) about to start.
pub(crate) fn copy_line(rel: &Path) -> Result<String> {
    Ok(json(&rel.to_string_lossy())? + "\n")
}

fn json(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(|e| Error::Other(e.to_string()))
}

/// The files the partial copy in `dir` was made of, relative to `dir`.
pub(crate) fn copies(dir: &Path) -> Result<BTreeSet<PathBuf>> {
    let text = fs::read_to_string(dir.join(MOVE_MARKER))?;
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    let listed = match serde_json::from_str::<Header>(first) {
        // A line cut short by the kill named a copy that hadn't started.
        Ok(_) => lines
            .filter_map(|l| serde_json::from_str::<String>(l).ok())
            .map(PathBuf::from)
            .collect(),
        Err(_) => source_files(Path::new(first))?,
    };
    Ok(listed.into_iter().filter(|p| in_data_subdir(p)).collect())
}

fn source_files(from: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for sub in DATA_SUBDIRS.iter().filter(|_| from.is_absolute()) {
        let listed = list_files(&from.join(sub))?;
        files.extend(listed.into_iter().map(|f| Path::new(sub).join(f.rel)));
    }
    Ok(files)
}

fn in_data_subdir(rel: &Path) -> bool {
    let mut parts = rel.components();
    let top = parts.next().and_then(|c| c.as_os_str().to_str());
    top.is_some_and(|t| DATA_SUBDIRS.contains(&t))
        && rel.components().count() > 1
        && rel.components().all(|c| matches!(c, Component::Normal(_)))
}

/// Deletes the partial copy in a marked folder (not its marker) for a new move copying `plan`, unless the folder
/// also holds other top-level files, or files added by hand where `plan` copies one. Returns
/// the files added by hand to its subfolders, which stay.
pub(crate) fn reuse(dir: &Path, plan: &Plan) -> Result<BTreeSet<PathBuf>> {
    let mut foreign = Vec::new();
    for entry in fs::read_dir(dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name != MOVE_MARKER && !DATA_SUBDIRS.contains(&name.as_str()) {
            foreign.push(name);
        }
    }
    let copied = copies(dir)?;
    let added: BTreeSet<_> = added_by_hand(dir, &copied)?.into_iter().collect();
    // Compared ignoring case: on a case-insensitive volume the copy would overwrite them.
    let planned: BTreeSet<_> = planned(plan).iter().map(|p| folded(p)).collect();
    let mut clash: Vec<_> = (added.iter())
        .filter(|p| planned.contains(&folded(p)))
        .map(|p| p.display())
        .collect();
    if !foreign.is_empty() || !clash.is_empty() {
        let (what, mut names) = match foreign.is_empty() {
            false => ("other files besides the partial copy", foreign),
            true => ("files added by hand where the move copies its own", {
                clash.truncate(5);
                clash.iter().map(ToString::to_string).collect()
            }),
        };
        names.sort();
        return Err(Error::Other(format!(
            "{} holds {what} ({}); move them out of the way or remove the partial copy from the \
             storage status, then move there again",
            dir.display(),
            names.join(", ")
        )));
    }
    remove_copies(dir, &copied)?;
    Ok(added)
}

fn folded(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

/// Files under the app's subfolders of `dir` that are not among `copies`.
fn added_by_hand(dir: &Path, copies: &BTreeSet<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut added = Vec::new();
    for sub in DATA_SUBDIRS {
        for file in list_files(&dir.join(sub))? {
            let rel = Path::new(sub).join(file.rel);
            if !copies.contains(&rel) {
                added.push(rel);
            }
        }
    }
    Ok(added)
}

/// Total size of the `copies` still in `dir`.
pub(crate) fn size(dir: &Path, copies: &BTreeSet<PathBuf>) -> u64 {
    let len = |rel: &PathBuf| fs::symlink_metadata(dir.join(rel)).map_or(0, |m| m.len());
    copies.iter().map(len).sum()
}

/// Deletes `copies` from `dir`, then the folders that leaves empty. A path through a link is
/// skipped: the copy never made one, so it was put there by hand.
pub(crate) fn remove_copies(dir: &Path, copies: &BTreeSet<PathBuf>) -> Result<()> {
    for rel in copies.iter().filter(|rel| !through_link(dir, rel)) {
        match fs::remove_file(dir.join(rel)) {
            // Not a directory: a file added by hand where the copy's folder would go.
            Err(e) if !matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
                return Err(e.into())
            }
            _ => {}
        }
    }
    let mut parents: Vec<&Path> = copies
        .iter()
        .flat_map(|rel| rel.ancestors().skip(1))
        .filter(|p| !p.as_os_str().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    parents.sort_by_key(|p| Reverse(p.components().count()));
    for parent in parents {
        fs::remove_dir(dir.join(parent)).ok();
    }
    Ok(())
}

fn through_link(dir: &Path, rel: &Path) -> bool {
    rel.ancestors()
        .skip(1)
        .filter(|p| !p.as_os_str().is_empty())
        .any(|p| fs::symlink_metadata(dir.join(p)).is_ok_and(|m| m.is_symlink()))
}
