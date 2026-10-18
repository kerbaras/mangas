//! Covers that are cut short or damaged, in a download or on disk, never get stuck in the cache.

use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use image::ImageFormat;
use mangas_core::covers::{cover, CoverCache, Limits};
use mangas_core::domain::Result;
use mangas_core::download::PageFetcher;

fn encoded(format: ImageFormat) -> Vec<u8> {
    let img = image::RgbImage::from_fn(48, 72, |x, y| {
        image::Rgb([(x * 5) as u8, (y * 3) as u8, (x ^ y) as u8])
    });
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, format)
        .unwrap();
    out.into_inner()
}

fn png() -> Vec<u8> {
    encoded(ImageFormat::Png)
}

fn jpeg() -> Vec<u8> {
    encoded(ImageFormat::Jpeg)
}

struct Fixed {
    body: Vec<u8>,
    calls: AtomicUsize,
}

#[async_trait]
impl PageFetcher for Fixed {
    async fn fetch(&self, _: &str) -> Result<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.body.clone())
    }
}

fn cache(dir: &Path) -> CoverCache {
    CoverCache::new(dir, Limits::default())
}

fn files(dir: &Path) -> usize {
    std::fs::read_dir(dir).map(|d| d.count()).unwrap_or(0)
}

#[test]
fn truncated_or_corrupt_downloads_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let c = cache(dir.path());
    for whole in [png(), jpeg()] {
        let cut = &whole[..whole.len() / 2];
        assert!(c.store("comick", "m", "https://c/x", cut).is_err());
        let mut garbled = whole.clone();
        garbled[40..].fill(0);
        assert!(c.store("comick", "m", "https://c/x", &garbled).is_err());
    }
    assert_eq!(files(dir.path()), 0);
}

#[tokio::test]
async fn a_corrupt_file_on_disk_is_dropped_and_fetched_again() {
    let dir = tempfile::tempdir().unwrap();
    let fetcher = Fixed {
        body: png(),
        calls: AtomicUsize::new(0),
    };
    let url = "https://cdn/a.png";
    let path = cover(&cache(dir.path()), &fetcher, "comick", "m1", url)
        .await
        .unwrap();
    let whole = std::fs::read(&path).unwrap();
    // Cut short on disk, overwritten with the same length, and one byte flipped mid-body.
    let mut flipped = whole.clone();
    flipped[whole.len() / 2] ^= 0xFF;
    let damage = [
        whole[..whole.len() / 2].to_vec(),
        vec![0; whole.len()],
        flipped,
    ];
    for (i, bad) in damage.iter().enumerate() {
        std::fs::write(&path, bad).unwrap();
        assert!(cache(dir.path()).lookup("comick", "m1", url).is_none());
        let again = cover(&cache(dir.path()), &fetcher, "comick", "m1", url)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&again).unwrap(), whole);
        assert_eq!(fetcher.calls.load(Ordering::SeqCst), i + 2);
    }
}

#[test]
fn a_jpeg_with_trailing_padding_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let mut padded = jpeg();
    padded.extend([0u8; 200]);
    assert!(cache(dir.path())
        .store("comick", "m", "https://c/x", &padded)
        .is_ok());
}

#[test]
fn a_rewrite_of_the_same_url_leaves_only_the_latest_body() {
    let dir = tempfile::tempdir().unwrap();
    let c = cache(dir.path());
    c.store("comick", "m", "https://c/x", &png()).unwrap();
    let last = c.store("comick", "m", "https://c/x", &jpeg()).unwrap();
    assert_eq!(files(dir.path()), 1);
    assert_eq!(c.lookup("comick", "m", "https://c/x"), Some(last));
}
