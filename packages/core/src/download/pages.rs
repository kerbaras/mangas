use std::collections::HashSet;
use std::path::Path;

use futures::stream::{self, StreamExt};

use super::cbz::is_image;
use super::event::{Event, OnEvent};
use super::fetcher::PageFetcher;
use super::incomplete::{incomplete, missing_on_disk};
use super::plan::{page_file_name_for, plan, PageJob};
use crate::domain::{ChapterId, Result};
use crate::sources::PageUrls;

/// Pages fetched at the same time within one chapter.
pub const CONCURRENCY: usize = 4;

/// What to do with the page files already in the chapter directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnDisk {
    /// Resume: a page already on disk is not fetched again.
    #[default]
    Keep,
    /// Download again: every page is fetched and overwrites its file; page images it did not
    /// write are removed once the chapter is complete.
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
/// `on_disk` is [`OnDisk::Replace`]); a complete chapter leaves no `.part` file behind. Emits
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
    remove_files(dest, is_page_partial)?;
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
    let results: Vec<_> = stream::iter(jobs)
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
        .collect()
        .await;
    let mut written = HashSet::new();
    let mut failed = Vec::new();
    for (index, result) in results {
        match result {
            Ok(name) => {
                written.insert(name);
            }
            Err(e) => failed.push((index, e)),
        }
    }
    if !failed.is_empty() {
        return Err(incomplete(failed, total));
    }
    let missing = plan(urls, &existing_files(dest)?).len();
    if missing > 0 {
        return Err(missing_on_disk(missing, total));
    }
    if on_disk == OnDisk::Replace {
        // Page images this full re-download did not write (another extension, a page the
        // source no longer has) would end up in an export as old or duplicate pages.
        remove_files(dest, |name| is_image(name) && !written.contains(name))?;
    }
    // The chapter is complete and this run holds its folder: any `.part` left is stale.
    remove_files(dest, |name| name.ends_with(".part"))?;
    Ok(total)
}

/// Removes the files in `dest` whose name matches `drop`.
fn remove_files(dest: &Path, drop: impl Fn(&str) -> bool) -> Result<()> {
    for name in existing_files(dest)? {
        if drop(&name) {
            std::fs::remove_file(dest.join(name))?;
        }
    }
    Ok(())
}

/// A page `.part` file (`0007.png.part`) left by an interrupted run. Its name came from that
/// run's bytes (or the URL, before pages were named by format), so this run may never
/// overwrite it; a run drops these before it starts, other `.part` files once it completes.
fn is_page_partial(name: &str) -> bool {
    name.strip_suffix(".part").is_some_and(is_page_name)
}

/// A page file name: digits, a dot, any extension (an unknown body keeps the URL's).
fn is_page_name(name: &str) -> bool {
    let numbered = |stem: &str| !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit());
    name.rsplit_once('.')
        .is_some_and(|(stem, _)| numbered(stem))
}

/// Writes to `<name>.part` first so an interrupted download never leaves a truncated page
/// that a later resume would skip. The file is named by the image's real format; returns that
/// name.
async fn fetch_page(fetcher: &dyn PageFetcher, job: &PageJob, dest: &Path) -> Result<String> {
    let bytes = fetcher.fetch(&job.url).await?;
    let name = page_file_name_for(job.index, &job.url, &bytes);
    let final_path = dest.join(&name);
    let part_path = dest.join(format!("{name}.part"));
    tokio::fs::write(&part_path, bytes).await?;
    tokio::fs::rename(&part_path, &final_path).await?;
    Ok(name)
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
