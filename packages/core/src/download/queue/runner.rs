use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::job::Job;
use crate::domain::{Error, Result};
use crate::download::{download_into_library, OnDisk, OnEvent, PageFetcher};
use crate::library::{self, Library};
use crate::sources::Source;

/// Downloads the chapter of one job. The apps use `mangas_providers::QueueRunner` (the
/// provider with its stored session, the page client, the downloads folder); tests use fakes.
/// Page progress is reported through `on_event` as [`crate::download::Event::PageDone`].
#[async_trait]
pub trait JobRunner: Send + Sync {
    async fn run(&self, lib: &Library, job: &Job, on_event: OnEvent<'_>) -> Result<()>;
}

/// The library's chapter for `job` downloaded under `root` (resuming pages already there, or
/// fetching all of them again for a [`Job::force`] job) and marked downloaded. Returns the chapter directory.
pub async fn run_job(
    lib: &Library,
    source: &dyn Source,
    fetcher: &dyn PageFetcher,
    job: &Job,
    root: &Path,
    on_event: OnEvent<'_>,
) -> Result<PathBuf> {
    let chapter = library::list_chapters(lib, &job.source, &job.manga_id, None)?
        .into_iter()
        .find(|c| c.id == job.chapter_id)
        .ok_or_else(|| Error::NotFound(format!("chapter {} in the library", job.chapter_id)))?;
    let on_disk = if job.force {
        OnDisk::Replace
    } else {
        OnDisk::Keep
    };
    download_into_library(source, fetcher, lib, &chapter, root, on_disk, on_event).await
}
