//! Replacing a book written in parts keeps the old parts when the new set fails (#134), and the
//! size warning measures the book with the most pages.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{estimate_largest, export, ExportRequest};
use mangas_core::export::{profile, Format, PageSettings};
use mangas_core::smartcut::Preset;

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

fn request(root: &Path, split: Option<u64>) -> ExportRequest {
    ExportRequest {
        chapter_dirs: vec![chapter(root, "Chapter 1", 3), chapter(root, "Chapter 2", 3)],
        device: profile("kindle-paperwhite12").unwrap(),
        format: Format::Epub,
        cut: Preset::Balanced.options(),
        page: PageSettings::default(),
        title: "Big".into(),
        title_explicit: false,
        output: root.join("out").join("Big.epub"),
        replace: true,
        meta: Default::default(),
        labels: Default::default(),
        split,
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn replacing_parts_renames_the_new_set_over_the_old_and_leaves_no_staging_file() {
    let root = tempfile::tempdir().unwrap();
    let whole = export(&request(root.path(), None), None, &mut |_| {}).unwrap();
    let max = std::fs::metadata(&whole.output).unwrap().len() / 2;
    std::fs::remove_file(&whole.output).unwrap();
    let req = request(root.path(), Some(max));
    let first = export(&req, None, &mut |_| {}).unwrap();
    for part in &first.parts {
        std::fs::write(&part.path, b"old").unwrap();
    }
    let again = export(&req, None, &mut |_| {}).unwrap();
    assert_eq!(again.outputs(), first.outputs());
    for part in &again.parts {
        assert_ne!(std::fs::read(&part.path).unwrap(), b"old");
    }
    let expected: Vec<String> = (1..=first.parts.len())
        .map(|n| format!("Big (part {n}).epub"))
        .collect();
    assert_eq!(names(&root.path().join("out")), expected);
}

#[test]
fn the_estimate_measures_the_book_with_the_most_pages_not_the_most_chapters() {
    let root = tempfile::tempdir().unwrap();
    let short: Vec<PathBuf> = (1..=3)
        .map(|n| chapter(root.path(), &format!("Short {n}"), 1))
        .collect();
    let long = vec![chapter(root.path(), "Long", 8)];
    let device = profile("kindle-paperwhite12").unwrap();
    let cut = Preset::Balanced.options();
    let page = PageSettings::default();
    let guess = estimate_largest(&[short, long], device, cut, page).unwrap();
    assert_eq!(guess.pages, 8);
}
