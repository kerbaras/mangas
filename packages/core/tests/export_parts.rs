//! Books over a part size are written as numbered parts (#134); the JPEG quality and the size
//! estimate that lets the dialog and CLI warn before writing.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{estimate, export, ExportRequest, ExportStats};
use mangas_core::export::{profile, ExportPreset, Format, PageSettings};
use mangas_core::library::Settings;
use mangas_core::smartcut::Preset;

/// Noisy pages, so their JPEGs are as big as real scans.
fn chapter(root: &Path, name: &str, pages: usize) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let mut seed = 7u32;
    for i in 0..pages {
        let img = RgbaImage::from_fn(800, 1200, |_, _| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let v = (seed >> 24) as u8;
            Rgba([v, v, v, 255])
        });
        img.save(dir.join(format!("{:03}.png", i + 1))).unwrap();
    }
    dir
}

fn request(root: &Path, page: PageSettings, split: Option<u64>) -> ExportRequest {
    ExportRequest {
        chapter_dirs: vec![chapter(root, "Chapter 1", 3), chapter(root, "Chapter 2", 3)],
        device: profile("kindle-paperwhite12").unwrap(),
        format: Format::Epub,
        cut: Preset::Balanced.options(),
        page,
        title: "Big".into(),
        title_explicit: false,
        output: root.join("Big.epub"),
        replace: false,
        meta: Default::default(),
        labels: Default::default(),
        split,
    }
}

fn run(req: &ExportRequest) -> ExportStats {
    export(req, None, &mut |_| {}).unwrap()
}

fn size(path: &Path) -> u64 {
    std::fs::metadata(path).unwrap().len()
}

#[test]
fn a_book_over_the_part_size_is_written_as_numbered_parts_under_it() {
    let root = tempfile::tempdir().unwrap();
    let whole = run(&request(root.path(), PageSettings::default(), None));
    let total = size(&whole.output);
    let max = total / 2;
    let stats = run(&request(root.path(), PageSettings::default(), Some(max)));
    assert!(stats.parts.len() >= 2, "{:?}", stats.parts);
    assert_eq!(stats.output, stats.parts[0].path);
    assert_eq!(
        stats.parts.iter().map(|p| p.pages).sum::<usize>(),
        stats.pages
    );
    for (i, part) in stats.parts.iter().enumerate() {
        let name = part
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(name, format!("Big (part {}).epub", i + 1));
        assert!(
            size(&part.path) <= max,
            "{name}: {} > {max}",
            size(&part.path)
        );
    }
    assert!(!root.path().join("Big (2).epub").exists());
}

#[test]
fn a_book_under_the_part_size_stays_one_file() {
    let root = tempfile::tempdir().unwrap();
    let stats = run(&request(
        root.path(),
        PageSettings::default(),
        Some(u64::MAX),
    ));
    assert!(stats.parts.is_empty());
    assert_eq!(stats.outputs(), vec![root.path().join("Big.epub")]);
}

#[test]
fn a_lower_jpeg_quality_makes_a_smaller_book() {
    let root = tempfile::tempdir().unwrap();
    let full = size(&run(&request(root.path(), PageSettings::default(), None)).output);
    let low = PageSettings {
        quality: Some(40),
        ..Default::default()
    };
    let small = size(&run(&request(root.path(), low, None)).output);
    assert!(small * 10 < full * 8, "quality 40: {small} B vs {full} B");
}

#[test]
fn the_estimate_is_close_to_the_written_size() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path(), PageSettings::default(), None);
    let guess = estimate(&req.chapter_dirs, req.device, req.cut, req.page).unwrap();
    let stats = run(&req);
    assert_eq!(guess.pages, stats.pages);
    let real = size(&stats.output) as f64;
    let off = (guess.bytes as f64 - real).abs() / real;
    assert!(off < 0.15, "estimate {} B vs {real} B", guess.bytes);
}

fn entry(path: &Path, name: &str) -> String {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    std::io::read_to_string(zip.by_name(name).unwrap()).unwrap()
}

#[test]
fn a_part_starting_inside_a_chapter_opens_with_its_title_and_is_named_n_of_total() {
    let root = tempfile::tempdir().unwrap();
    let whole = run(&request(root.path(), PageSettings::default(), None));
    assert_eq!(whole.pages, 6);
    let stats = run(&request(
        root.path(),
        PageSettings::default(),
        Some(size(&whole.output) / 3),
    ));
    let total = stats.parts.len();
    assert!(total >= 3, "{:?}", stats.parts);
    let mut start = 0;
    for (i, part) in stats.parts.iter().enumerate() {
        let opf = entry(&part.path, "OEBPS/content.opf");
        assert!(opf.contains(&format!(
            "<dc:title>Big (part {} of {total})</dc:title>",
            i + 1
        )));
        let nav = entry(&part.path, "OEBPS/nav.xhtml");
        let first = nav.split("<li><a href=").nth(1).unwrap();
        let chapter = if start < 3 { "Chapter 1" } else { "Chapter 2" };
        assert!(
            first.contains(&format!(">{chapter}</a>")),
            "part {}: {nav}",
            i + 1
        );
        start += part.pages;
    }
}

#[test]
fn the_quality_stored_for_the_profile_applies_unless_a_layer_sets_one() {
    let stored = Settings {
        jpeg_quality: [("kobo-clara".to_string(), 60)].into(),
        ..Default::default()
    };
    let on = |profile: Option<&str>, q: Option<u8>| ExportPreset {
        profile: profile.map(String::from),
        jpeg_quality: q,
        ..Default::default()
    };
    let quality = |p: ExportPreset| p.with_stored_quality(&stored, "kobo-clara").jpeg_quality;
    assert_eq!(quality(on(None, None)), Some(60));
    assert_eq!(quality(on(Some("kobo-clara"), Some(80))), Some(80));
    assert_eq!(quality(on(Some("kindle-paperwhite12"), None)), None);
}
