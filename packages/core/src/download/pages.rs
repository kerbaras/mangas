use std::collections::HashSet;
use std::path::Path;

use futures::stream::{self, StreamExt};

use super::cbz::is_image;
use super::event::{Event, OnEvent};
use super::fetcher::PageFetcher;
use super::incomplete::{incomplete, missing_on_disk};
use crate::domain::{ChapterId, Result};
use crate::sources::PageUrls;

/// Pages fetched at the same time within one chapter.
pub const CONCURRENCY: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageJob {
    /// 1-based page index.
    pub index: usize,
    pub url: String,
    pub file_name: String,
}

/// `0001.png` style file name for page `index` (1-based), keeping the source's extension.
pub fn page_file_name(index: usize, url: &str) -> String {
    format!("{index:04}.{}", extension_of(url))
}

/// Pages still to fetch given the files already in the chapter directory (resume). A page
/// counts as present by its index, whatever its extension: `0003.png` from a full-quality run
/// is page 3 even when the URL now ends in `.jpg` (data saver), so no duplicate gets written.
pub fn plan(urls: &PageUrls, existing: &HashSet<String>) -> Vec<PageJob> {
    let present = present_indexes(existing);
    urls.0
        .iter()
        .enumerate()
        .map(|(i, url)| PageJob {
            index: i + 1,
            url: url.clone(),
            file_name: page_file_name(i + 1, url),
        })
        .filter(|job| !present.contains(&job.index))
        .collect()
}

/// Indexes of the finished page images among `names` (`0003.png`, not `0003.png.part` or
/// `0003.txt`).
fn present_indexes(names: &HashSet<String>) -> HashSet<usize> {
    names
        .iter()
        .filter(|name| is_image(name))
        .filter_map(|name| {
            let (stem, _) = name.rsplit_once('.')?;
            let numbered = !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit());
            numbered.then(|| stem.parse().ok()).flatten()
        })
        .collect()
}

/// What to do with the page files already in the chapter directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnDisk {
    /// Resume: a page already on disk is not fetched again.
    #[default]
    Keep,
    /// Download again: every page is fetched and overwrites its file.
    Replace,
}

/// [`download_pages_with`] resuming the pages already on disk.
pub async fn download_pages(
    fetcher: &dyn PageFetcher,
    chapter: &ChapterId,
    urls: &PageUrls,
    dest: &Path,
    on_event: OnEvent<'_>,
) -> Result<usize> {
    download_pages_with(fetcher, chapter, urls, dest, OnDisk::Keep, on_event).await
}

/// Downloads every page of `urls` into `dest`, skipping files that already exist (unless
/// `on_disk` is [`OnDisk::Replace`]). Emits
/// [`Event::PageDone`] per page (including skipped ones) and returns the page count. A page
/// that fails does not stop the others (they stay on disk for a resume), but the chapter is
/// an error unless all `urls.len()` pages end up on disk.
pub async fn download_pages_with(
    fetcher: &dyn PageFetcher,
    chapter: &ChapterId,
    urls: &PageUrls,
    dest: &Path,
    on_disk: OnDisk,
    on_event: OnEvent<'_>,
) -> Result<usize> {
    tokio::fs::create_dir_all(dest).await?;
    let existing = match on_disk {
        OnDisk::Keep => existing_files(dest)?,
        OnDisk::Replace => HashSet::new(),
    };
    let total = urls.0.len();
    let jobs = plan(urls, &existing);
    for index in (1..=total).filter(|i| !jobs.iter().any(|j| j.index == *i)) {
        on_event(Event::PageDone {
            chapter: chapter.clone(),
            page: index,
            total,
        });
    }
    let failed: Vec<_> = stream::iter(jobs)
        .map(|job| async move {
            let result = fetch_page(fetcher, &job, dest).await;
            if result.is_ok() {
                on_event(Event::PageDone {
                    chapter: chapter.clone(),
                    page: job.index,
                    total,
                });
            }
            (job.index, result)
        })
        .buffer_unordered(CONCURRENCY)
        .filter_map(|(index, result)| async move { result.err().map(|e| (index, e)) })
        .collect()
        .await;
    if !failed.is_empty() {
        return Err(incomplete(failed, total));
    }
    let missing = plan(urls, &existing_files(dest)?).len();
    if missing > 0 {
        return Err(missing_on_disk(missing, total));
    }
    Ok(total)
}

/// Writes to `<name>.part` first so an interrupted download never leaves a truncated page
/// that a later resume would skip.
async fn fetch_page(fetcher: &dyn PageFetcher, job: &PageJob, dest: &Path) -> Result<()> {
    let bytes = fetcher.fetch(&job.url).await?;
    let final_path = dest.join(&job.file_name);
    let part_path = dest.join(format!("{}.part", job.file_name));
    tokio::fs::write(&part_path, bytes).await?;
    tokio::fs::rename(&part_path, &final_path).await?;
    Ok(())
}

fn existing_files(dest: &Path) -> Result<HashSet<String>> {
    let mut names = HashSet::new();
    for entry in std::fs::read_dir(dest)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            names.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(names)
}

fn extension_of(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let file = path.rsplit('/').next().unwrap_or(path);
    file.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .filter(|ext| {
            !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or_else(|| "jpg".to_string())
}
