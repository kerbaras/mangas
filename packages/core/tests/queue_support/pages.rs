//! A real chapter download for the queue tests: pages served from memory, written to disk by
//! the core's resuming downloader.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, Result};
use mangas_core::download::queue::{run_job, Job, JobRunner};
use mangas_core::download::{OnEvent, PageFetcher};
use mangas_core::library::Library;
use mangas_core::sources::{PageUrls, Source};

/// Pages from memory; while `hang` is set, every page from index 3 on never arrives.
#[derive(Default)]
pub struct Fetcher {
    pub hang: AtomicBool,
    pub fetched: Mutex<Vec<String>>,
}

#[async_trait]
impl PageFetcher for Fetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        let index: usize = url.rsplit('/').next().unwrap().parse().unwrap();
        if self.hang.load(Ordering::SeqCst) && index >= 3 {
            std::future::pending::<()>().await;
        }
        self.fetched.lock().unwrap().push(url.to_string());
        Ok(url.as_bytes().to_vec())
    }
}

/// A chapter of six pages, `https://cdn/1` to `https://cdn/6`.
struct Pages;

#[async_trait]
impl Source for Pages {
    fn name(&self) -> &'static str {
        "a"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>> {
        Ok(vec![])
    }
    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        Err(Error::NotFound(id.0.clone()))
    }
    async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>> {
        Ok(vec![])
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls> {
        Ok(PageUrls(
            (1..=6).map(|i| format!("https://cdn/{i}")).collect(),
        ))
    }
}

/// The real chapter download (resuming pages on disk) into `root`.
pub struct Runner(pub Arc<Fetcher>, pub PathBuf);

#[async_trait]
impl JobRunner for Runner {
    async fn run(&self, lib: &Library, job: &Job, on_event: OnEvent<'_>) -> Result<()> {
        run_job(lib, &Pages, self.0.as_ref(), job, &self.1, on_event).await?;
        Ok(())
    }
}

/// Page files of the one chapter folder under `root`.
pub fn pages_on_disk(root: &Path) -> HashSet<String> {
    let dir = walk(root).into_iter().find(|p| p.is_dir() && has_pages(p));
    dir.map(|d| {
        std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| !n.ends_with(".part"))
            .collect()
    })
    .unwrap_or_default()
}

fn has_pages(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .unwrap()
        .any(|e| e.unwrap().path().is_file())
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let children: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let nested = children.iter().filter(|p| p.is_dir()).flat_map(|p| walk(p));
    children
        .iter()
        .cloned()
        .chain(nested.collect::<Vec<_>>())
        .collect()
}
