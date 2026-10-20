//! Shared fixtures for the export format tests: chapter folders and a full export run.
#![allow(dead_code)]

use std::io::Read;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{export, ExportRequest, ExportStats};
use mangas_core::export::{profile, Format, Labels};
use mangas_core::smartcut::Preset;

pub fn chapter(root: &Path, name: &str, pages: usize) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..pages {
        RgbaImage::from_pixel(800, 1200, Rgba([240, 240, 240, 255]))
            .save(dir.join(format!("{:03}.png", i + 1)))
            .unwrap();
    }
    dir
}

pub fn run(format: Format, device: &str, file: &str) -> (tempfile::TempDir, ExportStats) {
    run_labelled(format, device, file, Labels::default(), false)
}

pub fn run_labelled(
    format: Format,
    device: &str,
    file: &str,
    labels: Labels,
    title_explicit: bool,
) -> (tempfile::TempDir, ExportStats) {
    let root = tempfile::tempdir().unwrap();
    let dirs = vec![
        chapter(root.path(), "Chapter 1", 2),
        chapter(root.path(), "Chapter 2", 3),
    ];
    let req = ExportRequest {
        split: None,
        chapter_dirs: dirs,
        device: profile(device).unwrap(),
        format,
        cut: Preset::Balanced.options(),
        page: Default::default(),
        title: "Tom & Jerry".into(),
        output: root.path().join(file),
        replace: false,
        labels,
        title_explicit,
        meta: Default::default(),
    };
    let stats = export(&req, None, &mut |_| {}).unwrap();
    (root, stats)
}

pub fn comic_info(cbz: &Path) -> String {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(cbz).unwrap()).unwrap();
    let mut xml = String::new();
    zip.by_name("ComicInfo.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

pub fn tag_text(doc: &roxmltree::Document, tag: &str) -> Option<String> {
    doc.descendants()
        .find(|n| n.has_tag_name(tag))
        .and_then(|n| n.text())
        .map(str::to_owned)
}
