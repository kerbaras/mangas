//! The size estimate of a long-strip webtoon counts the pages the smart cut cuts, not the
//! strip's height over the page height (#422): panels that don't fit two to a page give a page
//! each, so the writer writes about twice the pages the height alone suggests.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{
    estimate, export, plan_budget, BudgetSpec, Delivery, ExportRequest,
};
use mangas_core::export::{profile, Format, PageSettings};
use mangas_core::smartcut::Preset;

const PANEL: u32 = 650;
const GUTTER: u32 = 60;

/// One 800 px wide strip of `panels` noisy panels between white gutters.
fn strip_chapter(root: &Path, panels: u32) -> PathBuf {
    let dir = root.join("Chapter 1");
    std::fs::create_dir_all(&dir).unwrap();
    let mut seed = 11u32;
    let img = RgbaImage::from_fn(800, panels * (PANEL + GUTTER) + GUTTER, |_, y| {
        if y % (PANEL + GUTTER) < GUTTER {
            return Rgba([255, 255, 255, 255]);
        }
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let v = (seed >> 24) as u8;
        Rgba([v, v, v, 255])
    });
    img.save(dir.join("001.png")).unwrap();
    dir
}

fn request(root: &Path) -> ExportRequest {
    ExportRequest {
        chapter_dirs: vec![strip_chapter(root, 10)],
        device: profile("kindle-paperwhite12").unwrap(),
        format: Format::Epub,
        cut: Preset::Balanced.options(),
        page: PageSettings::default(),
        title: "Strip".into(),
        title_explicit: false,
        output: root.join("Strip.epub"),
        replace: false,
        meta: Default::default(),
        labels: Default::default(),
        split: None,
    }
}

#[test]
fn a_long_strip_is_estimated_at_the_pages_the_smart_cut_writes() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path());
    let guess = estimate(&req.chapter_dirs, req.device, req.cut, req.page, Format::Epub).unwrap();
    let stats = export(&req, None, &mut |_| {}).unwrap();
    let height_only = (10 * (PANEL + GUTTER) + GUTTER).div_ceil(req.device.height) as usize;
    assert!(stats.pages > height_only, "{} pages", stats.pages);
    assert_eq!(guess.pages, stats.pages);
    let real = std::fs::metadata(&stats.output).unwrap().len() as f64;
    let off = (guess.bytes as f64 - real).abs() / real;
    assert!(off < 0.10, "estimate {} B vs {real} B", guess.bytes);
}

#[test]
fn a_plan_and_its_levers_count_the_pages_the_export_writes() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path());
    let tight = BudgetSpec {
        device: req.device,
        cut: req.cut,
        page: req.page,
        delivery: Delivery::Email,
        split: None,
        format: Format::Epub,
    };
    let plan = plan_budget(std::slice::from_ref(&req.chapter_dirs), tight).unwrap();
    let stats = export(&req, None, &mut |_| {}).unwrap();
    assert_eq!(plan[0].pages, stats.pages);
    let real = std::fs::metadata(&stats.output).unwrap().len();
    assert_eq!(plan[0].over_budget, real > Delivery::Email.budget());
}
