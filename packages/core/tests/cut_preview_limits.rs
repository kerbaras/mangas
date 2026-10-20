//! Smart-cut preview limits: no page past the limit is read, a late cancel still fails.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{device, preview, PreviewRequest};
use mangas_core::smartcut::Preset;

fn request(dir: &Path, out: &Path, limit: usize) -> PreviewRequest {
    PreviewRequest {
        chapter_dir: dir.to_path_buf(),
        device: device("kindle-paperwhite12").unwrap(),
        cut: Preset::Balanced.options(),
        page: Default::default(),
        limit,
        out_dir: out.to_path_buf(),
    }
}

/// A chapter of `files` regular 800×1200 pages.
fn page_chapter(root: &Path, files: usize) -> PathBuf {
    let dir = root.join("Pages");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..files {
        let img = RgbaImage::from_fn(800, 1200, |x, y| {
            Rgba([(x % 200) as u8, (y % 200) as u8, 90, 255])
        });
        img.save(dir.join(format!("{:03}.png", i + 1))).unwrap();
    }
    dir
}

#[test]
fn the_preview_never_reads_a_page_past_its_limit() {
    let root = tempfile::tempdir().unwrap();
    let dir = page_chapter(root.path(), 29);
    // Index 29 is outside the margin sample (every 2nd file of 30), like on a long chapter.
    std::fs::write(dir.join("030.png"), b"not a png").unwrap();
    let p = preview(&request(&dir, &root.path().join("p"), 2), None, &|| false);
    assert_eq!(p.unwrap().pages.len(), 2);
}

#[test]
fn a_preview_cancelled_during_its_last_batch_fails() {
    let root = tempfile::tempdir().unwrap();
    let dir = page_chapter(root.path(), 2);
    let calls = std::cell::Cell::new(0);
    let cancelled = || {
        calls.set(calls.get() + 1);
        calls.get() > 1
    };
    assert!(preview(&request(&dir, &root.path().join("p"), 4), None, &cancelled).is_err());
}
