//! What `mangas kindle` reads from the library about the chapter folders it is given.

use std::path::{Path, PathBuf};

use mangas_core::export::{ExportPreset, Labels};
use mangas_core::library::{self, Library};

use super::kindle_titles::{spellings, titles_in};
use crate::paths;

pub(super) type FromLibrary = (
    Labels,
    Option<String>,
    Vec<String>,
    Option<ExportPreset>,
    ExportPreset,
    Vec<String>,
);

/// Series/chapter/volume labels from the library (folders it doesn't know export unlabelled),
/// the first chapter's series for the default title (a mixed export lacks it in `Labels`), the
/// TOC entries, the series export preset when every folder belongs to one series, and below it
/// what the series' source says (its reading direction); several series give the way they all turn;
/// then who the book credits (nobody when the library names no one).
pub(super) fn from_library(chapters: &[PathBuf]) -> FromLibrary {
    paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| {
            let series = series_of(&lib, chapters)?;
            let (preset, source) = match &series {
                Some((source, id)) => (
                    library::get_export_preset(&lib, source, id)?,
                    library::source_direction(&lib, source, id)?,
                ),
                None => (None, shared_direction(&lib, chapters)?),
            };
            Ok((
                labels_in(&lib, chapters)?,
                first_series(&lib, chapters)?,
                titles_in(&lib, chapters)?,
                preset,
                ExportPreset {
                    direction: source,
                    ..Default::default()
                },
                creators_in(&lib, chapters)?,
            ))
        })
        .unwrap_or_else(|e: anyhow::Error| {
            eprintln!("warning: could not read the chapters' series and numbers: {e}");
            Default::default()
        })
}

#[cfg(test)]
pub(super) fn series_preset(
    lib: &Library,
    chapters: &[PathBuf],
) -> anyhow::Result<Option<ExportPreset>> {
    Ok(match series_of(lib, chapters)? {
        Some((source, id)) => library::get_export_preset(lib, &source, &id)?,
        None => None,
    })
}

/// Each folder in every spelling the library may store it under.
fn each_spelled(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Vec<Vec<PathBuf>>> {
    chapters
        .iter()
        .map(|c| spellings(lib, &[c.as_path()]))
        .collect()
}

fn as_refs(spelled: &[Vec<PathBuf>]) -> Vec<Vec<&Path>> {
    spelled
        .iter()
        .map(|s| s.iter().map(PathBuf::as_path).collect())
        .collect()
}

/// The creators of the series among `chapters` ([`library::export_creators`]).
fn creators_in(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    let spelled = each_spelled(lib, chapters)?;
    Ok(library::export_creators(lib, &as_refs(&spelled))?)
}

/// The one series every folder (in any of its spellings) belongs to.
fn series_of(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Option<(String, String)>> {
    let spelled = each_spelled(lib, chapters)?;
    Ok(library::series_of_spelled(lib, &as_refs(&spelled))?)
}

/// The way the several series of `chapters` all turn ([`library::shared_direction`]).
fn shared_direction(
    lib: &Library,
    chapters: &[PathBuf],
) -> anyhow::Result<Option<mangas_core::domain::ReadingDirection>> {
    let spelled = each_spelled(lib, chapters)?;
    Ok(library::shared_direction(lib, &as_refs(&spelled))?)
}

pub(super) fn first_series(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Option<String>> {
    Ok(labels_in(lib, &chapters[..chapters.len().min(1)])?.series)
}

pub(super) fn labels_in(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Labels> {
    let refs: Vec<&Path> = chapters.iter().map(PathBuf::as_path).collect();
    let spellings = spellings(lib, &refs)?;
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    Ok(library::export_labels(lib, &dirs, chapters.len())?)
}
