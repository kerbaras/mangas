//! Local import: chapter folders and CBZ files on disk become a `local` manga whose chapters
//! are stored in the download layout (`<root>/local/<manga id>/<chapter>/`) and marked as
//! downloaded, so the Kindle exporter reads them like any downloaded chapter.

mod names;
mod pages;
mod plan;
mod report;
mod scan;

use std::path::{Path, PathBuf};

pub use names::{natural_cmp, parse_chapter_name, slug, ParsedName};
pub use report::{ImportReport, ImportedChapter, Skipped};

use super::{get_manga, list_chapters, mark_downloaded, upsert_chapters, upsert_manga, Library};
use crate::domain::{Chapter, Error, Result};
use crate::download::{chapter_dir, unique_chapter_dir};

/// Source name of imported manga. Their chapters are always on disk; there is no provider.
pub const LOCAL_SOURCE: &str = "local";

/// Imported manga have no provider to refresh chapters from or download pages with.
pub fn is_local(source: &str) -> bool {
    source == LOCAL_SOURCE
}

#[derive(Debug, Clone)]
pub struct ImportRequest {
    /// One series folder, or chapter folders / `.cbz` files of one series.
    pub paths: Vec<PathBuf>,
    /// Series title; defaults to the folder name.
    pub title: Option<String>,
    pub language: String,
}

/// Imports `req.paths` into the library, copying pages under `root` (the downloads root).
/// Chapters already imported are skipped; an unreadable chapter is reported, not fatal.
pub fn import_paths(lib: &Library, root: &Path, req: &ImportRequest) -> Result<ImportReport> {
    let scan = scan::scan(&req.paths)?;
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map_or(scan.title, str::to_string);
    let manga = plan::manga_for(&title);
    let mut known = list_chapters(lib, LOCAL_SOURCE, &manga.id, None)?;
    let mut skipped = scan.skipped;
    let mut imported = Vec::new();
    let mut added = Vec::new();
    for planned in plan::plan(&manga.id, &req.language, scan.entries) {
        let (entry, chapter) = (planned.entry, planned.chapter);
        if on_disk(&known, &chapter) {
            skipped.push(Skipped::new(&entry.name, "already imported"));
            continue;
        }
        let dest = unique_chapter_dir(chapter_dir(root, LOCAL_SOURCE, &chapter), &chapter, &known);
        let reason = match pages::write_pages(&entry, &dest) {
            Ok(0) => "no images".to_string(),
            Ok(pages) => {
                let chapter = Chapter {
                    local_path: Some(dest.to_string_lossy().into_owned()),
                    ..chapter
                };
                imported.push(ImportedChapter {
                    number: chapter.number.clone(),
                    title: chapter.title.clone(),
                    pages,
                    path: dest,
                });
                known.push(chapter.clone());
                added.push(chapter);
                continue;
            }
            Err(e) => e.to_string(),
        };
        let _ = std::fs::remove_dir_all(&dest);
        skipped.push(Skipped::new(&entry.name, &reason));
    }
    if known.is_empty() {
        return Err(Error::NotFound(format!(
            "no chapters with images in {}",
            describe(&req.paths)
        )));
    }
    save(lib, &manga, &req.language, &added)?;
    let manga = get_manga(lib, LOCAL_SOURCE, &manga.id)?.unwrap_or(manga);
    Ok(ImportReport {
        manga,
        imported,
        skipped,
    })
}

fn on_disk(known: &[Chapter], chapter: &Chapter) -> bool {
    known.iter().any(|c| {
        c.id == chapter.id
            && c.local_path
                .as_deref()
                .is_some_and(|p| Path::new(p).is_dir())
    })
}

fn save(lib: &Library, manga: &crate::Manga, language: &str, added: &[Chapter]) -> Result<()> {
    if added.is_empty() {
        return Ok(());
    }
    upsert_manga(lib, manga)?;
    upsert_chapters(lib, LOCAL_SOURCE, &manga.id, language, added)?;
    for c in added {
        if let Some(path) = &c.local_path {
            mark_downloaded(lib, LOCAL_SOURCE, &c.id, Path::new(path))?;
        }
    }
    Ok(())
}

fn describe(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
