//! What `mangas kindle` reads from the library about the chapter folders it is given.

use std::path::{Path, PathBuf};

use mangas_core::export::{ExportPreset, Labels};
use mangas_core::library::{self, Library};

use super::kindle_titles::{spellings, titles_in};
use crate::paths;

pub(super) type FromLibrary = (Labels, Option<String>, Vec<String>, Option<ExportPreset>);

/// Series/chapter/volume labels from the library (folders it doesn't know export unlabelled),
/// the first chapter's series for the default title (a mixed export lacks it in `Labels`), the
/// TOC entries, and the series export preset when every folder belongs to one series.
pub(super) fn from_library(chapters: &[PathBuf]) -> FromLibrary {
    paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| {
            let titles = titles_in(&lib, chapters)?;
            let preset = series_preset(&lib, chapters)?;
            Ok((
                labels_in(&lib, chapters)?,
                first_series(&lib, chapters)?,
                titles,
                preset,
            ))
        })
        .unwrap_or_else(|e: anyhow::Error| {
            eprintln!("warning: could not read the chapters' series and numbers: {e}");
            (Labels::default(), None, Vec::new(), None)
        })
}

pub(super) fn series_preset(
    lib: &Library,
    chapters: &[PathBuf],
) -> anyhow::Result<Option<ExportPreset>> {
    let spelled: Vec<Vec<PathBuf>> = chapters.iter().map(|c| spellings(&[c.as_path()])).collect();
    let refs: Vec<Vec<&Path>> = spelled
        .iter()
        .map(|s| s.iter().map(PathBuf::as_path).collect())
        .collect();
    Ok(match library::series_of_spelled(lib, &refs)? {
        Some((source, id)) => library::get_export_preset(lib, &source, &id)?,
        None => None,
    })
}

pub(super) fn first_series(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Option<String>> {
    Ok(labels_in(lib, &chapters[..chapters.len().min(1)])?.series)
}

pub(super) fn labels_in(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Labels> {
    let refs: Vec<&Path> = chapters.iter().map(PathBuf::as_path).collect();
    let spellings = spellings(&refs);
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    Ok(library::export_labels(lib, &dirs, chapters.len())?)
}
