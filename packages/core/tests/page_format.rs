//! Page files are named by their real image format (magic bytes), not by the URL.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use async_trait::async_trait;
use image::{ImageFormat, RgbImage};
use mangas_core::domain::{ChapterId, Error};
use mangas_core::download::{download_pages, download_pages_with, page_file_name_for, OnDisk};
use mangas_core::download::{Event, PageFetcher};
use mangas_core::images::sniff_extension;
use mangas_core::sources::PageUrls;

fn encoded(format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbImage::new(2, 2).write_to(&mut out, format).unwrap();
    out.into_inner()
}

/// Serves a JPEG for every URL, whatever its extension says, and counts the fetches.
#[derive(Default)]
struct JpegEverywhere(std::sync::Mutex<usize>);

#[async_trait]
impl PageFetcher for JpegEverywhere {
    async fn fetch(&self, _url: &str) -> Result<Vec<u8>, Error> {
        *self.0.lock().unwrap() += 1;
        Ok(encoded(ImageFormat::Jpeg))
    }
}

fn png_urls(n: usize) -> PageUrls {
    PageUrls(
        (1..=n)
            .map(|i| format!("https://cdn.example/{i}.png"))
            .collect(),
    )
}

fn names(dir: &Path) -> HashSet<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn ignore(_: Event) {}

#[test]
fn magic_bytes_decide_the_extension() {
    assert_eq!(sniff_extension(&encoded(ImageFormat::Jpeg)), Some("jpg"));
    assert_eq!(sniff_extension(&encoded(ImageFormat::Png)), Some("png"));
    assert_eq!(sniff_extension(&encoded(ImageFormat::Gif)), Some("gif"));
    assert_eq!(sniff_extension(&encoded(ImageFormat::WebP)), Some("webp"));
    assert_eq!(
        sniff_extension(b"\0\0\0\x1cftypavif\0\0\0\0avifmif1"),
        Some("avif")
    );
    assert_eq!(sniff_extension(b"<!doctype html><title>404</title>"), None);
}

#[test]
fn unknown_bodies_keep_the_url_extension() {
    let jpeg = encoded(ImageFormat::Jpeg);
    assert_eq!(page_file_name_for(1, "https://x/1.png", &jpeg), "0001.jpg");
    assert_eq!(page_file_name_for(2, "https://x/2.png", b"???"), "0002.png");
    assert_eq!(page_file_name_for(3, "https://x/3", b"???"), "0003.jpg");
}

#[tokio::test]
async fn a_jpeg_behind_a_png_url_is_saved_as_jpg() {
    let dir = tempfile::tempdir().unwrap();
    let chapter = ChapterId("c".into());
    let fetcher = JpegEverywhere::default();
    download_pages(&fetcher, &chapter, &png_urls(2), dir.path(), &ignore)
        .await
        .unwrap();
    assert_eq!(
        names(dir.path()),
        ["0001.jpg", "0002.jpg"].map(String::from).into()
    );
}

#[tokio::test]
async fn folders_from_older_runs_resume_by_page_number() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("0001.png"), encoded(ImageFormat::Jpeg)).unwrap();
    let chapter = ChapterId("c".into());
    let fetcher = JpegEverywhere::default();
    let total = download_pages(&fetcher, &chapter, &png_urls(2), dir.path(), &ignore)
        .await
        .unwrap();
    assert_eq!(total, 2);
    assert_eq!(*fetcher.0.lock().unwrap(), 1, "page 1 is already on disk");
    assert_eq!(
        names(dir.path()),
        ["0001.png", "0002.jpg"].map(String::from).into()
    );
}

#[tokio::test]
async fn downloading_again_renames_misnamed_pages() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("0001.png"), encoded(ImageFormat::Jpeg)).unwrap();
    std::fs::write(dir.path().join("0002.png"), encoded(ImageFormat::Jpeg)).unwrap();
    let chapter = ChapterId("c".into());
    let fetcher = JpegEverywhere::default();
    let urls = png_urls(2);
    download_pages_with(
        &fetcher,
        &chapter,
        &urls,
        dir.path(),
        OnDisk::Replace,
        &ignore,
    )
    .await
    .unwrap();
    assert_eq!(
        names(dir.path()),
        ["0001.jpg", "0002.jpg"].map(String::from).into()
    );
}

#[tokio::test]
async fn partial_files_from_an_interrupted_run_are_removed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("0001.png.part"), b"half").unwrap();
    std::fs::write(dir.path().join("0001.bin.part"), b"half").unwrap();
    std::fs::write(dir.path().join("notes.part"), b"stray").unwrap();
    let fetcher = JpegEverywhere::default();
    let chapter = ChapterId("c".into());
    download_pages(&fetcher, &chapter, &png_urls(1), dir.path(), &ignore)
        .await
        .unwrap();
    assert_eq!(names(dir.path()), ["0001.jpg"].map(String::from).into());
}

#[tokio::test]
async fn an_unknown_part_file_stays_until_the_chapter_completes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.part"), b"stray").unwrap();
    std::fs::write(dir.path().join("0001.jpg"), encoded(ImageFormat::Jpeg)).unwrap();
    let chapter = ChapterId("c".into());
    let failing = PageUrls(vec!["https://cdn.example/1.png".into(), String::new()]);
    let incomplete = download_pages(&Missing, &chapter, &failing, dir.path(), &ignore).await;
    assert!(incomplete.is_err());
    assert!(dir.path().join("notes.part").exists(), "kept for a resume");

    let fetcher = JpegEverywhere::default();
    let urls = png_urls(2);
    download_pages_with(
        &fetcher,
        &chapter,
        &urls,
        dir.path(),
        OnDisk::Replace,
        &ignore,
    )
    .await
    .unwrap();
    let pages = ["0001.jpg", "0002.jpg"].map(String::from).into();
    assert_eq!(
        names(dir.path()),
        pages,
        "a forced re-download cleans up too"
    );
}

/// Every fetch fails: the chapter never completes.
struct Missing;

#[async_trait]
impl PageFetcher for Missing {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        Err(Error::NotFound(url.into()))
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_part_file_with_a_non_utf8_name_is_removed_too() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    let odd = dir
        .path()
        .join(std::ffi::OsStr::from_bytes(b"page\xff.part"));
    std::fs::write(&odd, b"stray").unwrap();
    let fetcher = JpegEverywhere::default();
    let chapter = ChapterId("c".into());
    download_pages(&fetcher, &chapter, &png_urls(1), dir.path(), &ignore)
        .await
        .unwrap();
    assert!(!odd.exists());
}
