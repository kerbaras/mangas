use std::path::{Path, PathBuf};

use super::event::{Event, OnEvent};
use super::fetcher::{HttpFetcher, PageFetcher};
use super::pages::download_pages;
use crate::domain::{Chapter, ChapterId, Error, MangaId, Result};
use crate::library::{self, Library};
use crate::sources::Source;

/// `<root>/<source>/<manga id>/<chapter number>`.
pub fn chapter_dir(root: &Path, source: &str, manga_id: &MangaId, number: &str) -> PathBuf {
    let safe: String = number
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    root.join(source).join(&manga_id.0).join(safe)
}

/// Downloads one chapter into `dest` over HTTP and returns the number of pages.
pub async fn download_chapter(
    source: &dyn Source,
    chapter: &Chapter,
    dest: &Path,
    on_event: OnEvent<'_>,
) -> Result<usize> {
    let fetcher = HttpFetcher::new()?;
    download_chapter_with(source, &fetcher, chapter, dest, on_event).await
}

/// Same as [`download_chapter`] with an explicit page fetcher (tests inject a fake one).
pub async fn download_chapter_with(
    source: &dyn Source,
    fetcher: &dyn PageFetcher,
    chapter: &Chapter,
    dest: &Path,
    on_event: OnEvent<'_>,
) -> Result<usize> {
    let urls = source.pages(&chapter.id).await?;
    if urls.0.is_empty() {
        return Err(Error::Source {
            name: source.name().to_string(),
            message: format!("chapter {} has no pages", chapter.number),
        });
    }
    let pages = download_pages(fetcher, &chapter.id, &urls, dest, on_event).await?;
    on_event(Event::ChapterDone {
        chapter: chapter.id.clone(),
        path: dest.to_string_lossy().into_owned(),
        pages,
    });
    Ok(pages)
}

/// Outcome of a multi-chapter download. Failures do not stop the remaining chapters.
#[derive(Debug, Default)]
pub struct Report {
    pub downloaded: Vec<PathBuf>,
    pub failed: Vec<(ChapterId, String)>,
}

/// Downloads `chapters` one after another under `root` and records each finished chapter in
/// the library. Emits [`Event::Error`] for a chapter that fails and carries on.
pub async fn download_chapters(
    source: &dyn Source,
    lib: &Library,
    chapters: &[Chapter],
    root: &Path,
    on_event: OnEvent<'_>,
) -> Result<Report> {
    let fetcher = HttpFetcher::new()?;
    let mut report = Report::default();
    for chapter in chapters {
        let dest = chapter_dir(root, source.name(), &chapter.manga_id, &chapter.number);
        match download_chapter_with(source, &fetcher, chapter, &dest, on_event).await {
            Ok(_) => {
                library::mark_downloaded(lib, source.name(), &chapter.id, &dest)?;
                report.downloaded.push(dest);
            }
            Err(e) => {
                on_event(Event::Error {
                    chapter: chapter.id.clone(),
                    message: e.to_string(),
                });
                report.failed.push((chapter.id.clone(), e.to_string()));
            }
        }
    }
    Ok(report)
}
