//! Finds the chapters in the paths given to an import (the only filesystem walk).

use std::path::{Path, PathBuf};

use super::names::{extension, natural_cmp};
use super::report::Skipped;
use crate::domain::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Folder,
    Cbz,
}

/// One chapter on disk: a folder of images or a CBZ archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
}

#[derive(Debug)]
pub struct Scan {
    /// Series title guessed from the folder names.
    pub title: String,
    pub entries: Vec<Entry>,
    pub skipped: Vec<Skipped>,
}

/// A single folder holding chapter folders or `.cbz` files is a series named after it.
/// Anything else (an image-only folder, a list of `.cbz` files and folders) is a list of
/// chapters of one series named after the parent folder of the first path.
pub fn scan(paths: &[PathBuf]) -> Result<Scan> {
    let first = paths
        .first()
        .ok_or_else(|| Error::Other("nothing to import".into()))?;
    for p in paths {
        if !p.exists() {
            return Err(Error::NotFound(p.display().to_string()));
        }
    }
    if let [dir] = paths {
        if dir.is_dir() {
            let (entries, skipped) = classify(&children(dir)?);
            if !entries.is_empty() {
                return Ok(finish(name_of(dir), entries, skipped));
            }
        }
    }
    let title = first
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().map(name_of))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Imported".to_string());
    let (entries, skipped) = classify(paths);
    Ok(finish(title, entries, skipped))
}

fn finish(title: String, mut entries: Vec<Entry>, skipped: Vec<Skipped>) -> Scan {
    entries.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    Scan {
        title,
        entries,
        skipped,
    }
}

fn children(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if !name_of(&path).starts_with('.') {
            out.push(path);
        }
    }
    Ok(out)
}

fn classify(paths: &[PathBuf]) -> (Vec<Entry>, Vec<Skipped>) {
    let mut entries = Vec::new();
    let mut skipped = Vec::new();
    for path in paths {
        let name = name_of(path);
        match kind_of(path) {
            Some(kind) => entries.push(Entry {
                name,
                path: path.clone(),
                kind,
            }),
            None => skipped.push(Skipped::new(&name, "not a chapter folder or .cbz file")),
        }
    }
    (entries, skipped)
}

fn kind_of(path: &Path) -> Option<EntryKind> {
    if path.is_dir() {
        return Some(EntryKind::Folder);
    }
    let ext = extension(&name_of(path))?;
    (path.is_file() && (ext == "cbz" || ext == "zip")).then_some(EntryKind::Cbz)
}

pub(super) fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
