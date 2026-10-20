//! A failed replacement of a book written in parts keeps every old part (#134). Alone in its
//! test binary: staged parts are named `.mangas-<pid>-<n>-<name>.part`, `n` counting every part
//! staged in this process, so the name of the part to block is known.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{export, ExportRequest};
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
fn a_failed_replacement_keeps_every_old_part() {
    let root = tempfile::tempdir().unwrap();
    let whole = export(&request(root.path(), None), None, &mut |_| {}).unwrap();
    let max = std::fs::metadata(&whole.output).unwrap().len() / 2;
    std::fs::remove_file(&whole.output).unwrap();
    let req = request(root.path(), Some(max));
    let first = export(&req, None, &mut |_| {}).unwrap();
    assert!(first.parts.len() >= 2);
    for (i, part) in first.parts.iter().enumerate() {
        std::fs::write(&part.path, format!("old {i}")).unwrap();
    }
    // A folder where the next run stages part 2 makes writing part 2 fail.
    let out = root.path().join("out");
    let n = first.parts.len() + 1;
    let pid = std::process::id();
    let blocker = out.join(format!(".mangas-{pid}-{n}-Big (part 2).epub.part"));
    std::fs::create_dir(&blocker).unwrap();
    assert!(export(&req, None, &mut |_| {}).is_err());
    for (i, part) in first.parts.iter().enumerate() {
        assert_eq!(
            std::fs::read_to_string(&part.path).unwrap(),
            format!("old {i}")
        );
    }
    std::fs::remove_dir(&blocker).unwrap();
    let mut expected: Vec<String> = (1..=first.parts.len())
        .map(|n| format!("Big (part {n}).epub"))
        .collect();
    expected.sort();
    assert_eq!(names(&out), expected);
}
