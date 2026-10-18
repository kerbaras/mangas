use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use mangas_core::covers::{cover, image_extension, CoverCache, Limits};
use mangas_core::domain::{Error, Result};
use mangas_core::download::PageFetcher;

/// Smallest valid bodies of each kind, by magic bytes.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
const JPEG: &[u8] = b"\xff\xd8\xff\xe0\0\x10JFIF\0";

struct Fixed {
    body: Vec<u8>,
    calls: AtomicUsize,
}

impl Fixed {
    fn new(body: &[u8]) -> Self {
        Self {
            body: body.to_vec(),
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl PageFetcher for Fixed {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.body.is_empty() {
            return Err(Error::Other(format!("HTTP 403 for {url}")));
        }
        Ok(self.body.clone())
    }
}

fn cache(dir: &std::path::Path) -> CoverCache {
    CoverCache::new(dir, Limits::default())
}

fn files(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn image_extension_comes_from_the_bytes() {
    assert_eq!(image_extension(PNG), Some("png"));
    assert_eq!(image_extension(JPEG), Some("jpg"));
    assert_eq!(image_extension(b"<!DOCTYPE html><title>403</title>"), None);
}

#[tokio::test]
async fn fetches_once_then_serves_the_cached_file() {
    let dir = tempfile::tempdir().unwrap();
    let fetcher = Fixed::new(PNG);
    let first = cover(
        &cache(dir.path()),
        &fetcher,
        "comick",
        "m1",
        "https://cdn/a.png",
    )
    .await
    .unwrap();
    let again = cover(
        &cache(dir.path()),
        &fetcher,
        "comick",
        "m1",
        "https://cdn/a.png",
    )
    .await
    .unwrap();
    assert_eq!(first, again);
    assert_eq!(first.extension().unwrap(), "png");
    assert_eq!(std::fs::read(&first).unwrap(), PNG);
    assert_eq!(fetcher.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_new_cache_on_the_same_dir_reuses_covers() {
    // What an app restart does: a fresh CoverCache over the same folder.
    let dir = tempfile::tempdir().unwrap();
    cache(dir.path())
        .store("mangadex", "m1", "https://u/c.jpg", JPEG)
        .unwrap();
    let fetcher = Fixed::new(PNG);
    let path = cover(
        &cache(dir.path()),
        &fetcher,
        "mangadex",
        "m1",
        "https://u/c.jpg",
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), JPEG);
    assert_eq!(fetcher.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failures_and_non_images_are_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let denied = Fixed::new(b"");
    let html = Fixed::new(b"<html>Forbidden</html>");
    assert!(
        cover(&cache(dir.path()), &denied, "comick", "m", "https://c/x")
            .await
            .is_err()
    );
    let err = cover(&cache(dir.path()), &html, "comick", "m", "https://c/x")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not an image"), "{err}");
    assert!(cache(dir.path())
        .lookup("comick", "m", "https://c/x")
        .is_none());
    assert!(files(dir.path()).is_empty());
}

#[test]
fn a_changed_cover_url_replaces_the_old_file() {
    let dir = tempfile::tempdir().unwrap();
    let c = cache(dir.path());
    c.store("webtoons", "t1", "https://p/old.jpg", JPEG)
        .unwrap();
    c.store("webtoons", "t2", "https://p/other.jpg", JPEG)
        .unwrap();
    let new = c.store("webtoons", "t1", "https://p/new.png", PNG).unwrap();
    assert!(c.lookup("webtoons", "t1", "https://p/old.jpg").is_none());
    assert_eq!(c.lookup("webtoons", "t1", "https://p/new.png"), Some(new));
    assert!(c.lookup("webtoons", "t2", "https://p/other.jpg").is_some());
    assert_eq!(files(dir.path()).len(), 2);
}

#[test]
fn the_cache_is_bounded_and_drops_the_least_recently_used() {
    let dir = tempfile::tempdir().unwrap();
    let c = CoverCache::new(
        dir.path(),
        Limits {
            max_files: 2,
            max_bytes: 1 << 20,
        },
    );
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let set_used = |p: &std::path::Path, t| {
        let f = std::fs::File::options().write(true).open(p).unwrap();
        f.set_modified(t).unwrap();
    };
    let a = c.store("s", "a", "u", PNG).unwrap();
    let b = c.store("s", "b", "u", PNG).unwrap();
    set_used(&a, old);
    set_used(&b, old - std::time::Duration::from_secs(60));
    // Showing "b" makes it the most recent, so storing "c" evicts "a".
    assert!(c.lookup("s", "b", "u").is_some());
    c.store("s", "c", "u", PNG).unwrap();
    assert!(c.lookup("s", "a", "u").is_none());
    assert!(c.lookup("s", "b", "u").is_some());
    assert!(c.lookup("s", "c", "u").is_some());
}
