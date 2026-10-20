//! Smart-cut preview: bounded, matches the export's pages, follows the page tweaks.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{
    device, export, preview, trim::trim_page, ExportRequest, PreviewRequest,
};
use mangas_core::export::{Format, PageSettings};
use mangas_core::smartcut::Preset;

/// A webtoon chapter: `files` strips of 800×6000 with dark bands to cut between.
fn strip_chapter(root: &Path, files: usize) -> PathBuf {
    let dir = root.join("Chapter 1");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..files {
        let img = RgbaImage::from_fn(800, 6000, |x, y| {
            let band = (y / 100) % 2 == 0 && (100..700).contains(&x);
            if band {
                Rgba([30, 30, 30, 255])
            } else {
                Rgba([250, 250, 250, 255])
            }
        });
        img.save(dir.join(format!("{:03}.png", i + 1))).unwrap();
    }
    dir
}

fn request(dir: &Path, out: &Path, limit: usize, page: PageSettings) -> PreviewRequest {
    PreviewRequest {
        chapter_dir: dir.to_path_buf(),
        device: device("kindle-paperwhite12").unwrap(),
        cut: Preset::Balanced.options(),
        page,
        limit,
        out_dir: out.to_path_buf(),
    }
}

fn export_pages(dir: &Path, root: &Path, page: PageSettings) -> usize {
    let req = ExportRequest {
        split: None,
        chapter_dirs: vec![dir.to_path_buf()],
        device: device("kindle-paperwhite12").unwrap(),
        format: Format::Cbz,
        cut: Preset::Balanced.options(),
        page,
        title: "t".into(),
        title_explicit: false,
        output: root.join("book.cbz"),
        replace: false,
        meta: Default::default(),
        labels: Default::default(),
    };
    export(&req, None, &mut |_| {}).unwrap().pages
}

#[test]
fn the_preview_stops_at_its_page_limit() {
    let root = tempfile::tempdir().unwrap();
    let dir = strip_chapter(root.path(), 3);
    let out = root.path().join("preview");
    let p = preview(&request(&dir, &out, 4, Default::default()), None, &|| false).unwrap();
    assert_eq!(p.pages.len(), 4);
    assert!(p.more);
    for (i, page) in p.pages.iter().enumerate() {
        assert_eq!(page.path, out.join(format!("page-{:03}.jpg", i + 1)));
        let img = image::open(&page.path).unwrap();
        assert_eq!((img.width(), img.height()), (page.width, page.height));
    }
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 4);
}

#[test]
fn a_short_chapter_previews_every_page_the_export_writes() {
    let root = tempfile::tempdir().unwrap();
    let dir = strip_chapter(root.path(), 1);
    let all = export_pages(&dir, root.path(), Default::default());
    let p = preview(
        &request(&dir, &root.path().join("p"), 100, Default::default()),
        None,
        &|| false,
    )
    .unwrap();
    assert_eq!(p.pages.len(), all);
    assert!(!p.more);
}

#[test]
fn shorter_pages_cut_the_strip_into_more_pages() {
    let root = tempfile::tempdir().unwrap();
    let dir = strip_chapter(root.path(), 2);
    let short = PageSettings {
        height: Some(0.5),
        ..Default::default()
    };
    let full = export_pages(&dir, root.path(), Default::default());
    let halves = export_pages(&dir, root.path(), short);
    assert!(halves > full, "{halves} pages at half height vs {full}");
    let p = preview(
        &request(&dir, &root.path().join("p"), 2, short),
        None,
        &|| false,
    )
    .unwrap();
    let dev = device("kindle-paperwhite12").unwrap();
    assert!(p.pages[0].height <= dev.height / 2 + 1, "{:?}", p.pages[0]);
}

#[test]
fn a_cancelled_preview_fails() {
    let root = tempfile::tempdir().unwrap();
    let dir = strip_chapter(root.path(), 1);
    let req = request(&dir, &root.path().join("p"), 3, Default::default());
    assert!(preview(&req, None, &|| true).is_err());
}

#[test]
fn trimming_crops_blank_borders_but_keeps_the_art() {
    let page = RgbaImage::from_fn(800, 1200, |x, y| {
        let art = (100..700).contains(&x) && (50..1150).contains(&y);
        if art {
            Rgba([20, 20, 20, 255])
        } else {
            Rgba([255, 255, 255, 255])
        }
    });
    let trimmed = trim_page(&page);
    assert_eq!(trimmed.dimensions(), (600, 1100));
    let blank = RgbaImage::from_pixel(800, 1200, Rgba([255, 255, 255, 255]));
    assert_eq!(trim_page(&blank).dimensions(), (800, 1200));
    let wide = RgbaImage::from_fn(800, 1200, |x, _| {
        if (390..410).contains(&x) {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([255, 255, 255, 255])
        }
    });
    assert_eq!(trim_page(&wide).width(), 400, "at most a quarter per side");
}

#[test]
fn trimmed_strips_still_export_and_keep_more_of_the_screen_for_art() {
    let root = tempfile::tempdir().unwrap();
    let dir = strip_chapter(root.path(), 2);
    let trim = PageSettings {
        trim_margins: true,
        ..Default::default()
    };
    let plain = export_pages(&dir, root.path(), Default::default());
    let trimmed = export_pages(&dir, root.path(), trim);
    assert!(
        trimmed > plain,
        "wider art fills the screen sooner: {trimmed} vs {plain}"
    );
}
