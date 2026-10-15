//! Chapter directories -> EPUB through `export::kindle::export`.

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{device, export, ExportRequest, Progress};
use mangas_core::smartcut::Preset;

fn write_png(dir: &std::path::Path, name: &str, w: u32, h: u32) {
    RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]))
        .save(dir.join(name))
        .unwrap();
}

#[test]
fn exports_chapters_and_skips_empty_directories() {
    let root = tempfile::tempdir().unwrap();
    let ch1 = root.path().join("ch1");
    let empty = root.path().join("ch2");
    let ch3 = root.path().join("ch3");
    for d in [&ch1, &empty, &ch3] {
        std::fs::create_dir_all(d).unwrap();
    }
    write_png(&ch1, "001.png", 800, 1200);
    write_png(&ch1, "002.png", 800, 1200);
    std::fs::write(ch1.join("notes.txt"), b"ignored").unwrap();
    write_png(&ch3, "001.png", 800, 1200);
    let out = root.path().join("book.epub");
    let req = ExportRequest {
        chapter_dirs: vec![ch1, empty, ch3],
        device: device("kindle-paperwhite12").unwrap(),
        cut: Preset::Balanced.options(),
        title: "Book".into(),
        output: out.clone(),
    };
    let mut events = Vec::new();
    let stats = export(&req, None, &mut |p: Progress| events.push(p)).unwrap();
    assert_eq!(stats.pages, 3);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Progress::Chapter { .. }))
            .count(),
        3
    );
    assert!(matches!(
        events.last(),
        Some(Progress::Writing { pages: 3 })
    ));

    let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let mut nav = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("OEBPS/nav.xhtml").unwrap(), &mut nav).unwrap();
    assert!(nav.contains(">ch1<") && nav.contains(">ch3<"), "{nav}");
    assert!(
        !nav.contains(">ch2<"),
        "empty chapter has no TOC entry: {nav}"
    );
    assert!(
        nav.contains("text/page-00002.xhtml"),
        "ch3 starts at page 3: {nav}"
    );
}

#[test]
fn missing_directory_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    let req = ExportRequest {
        chapter_dirs: vec![root.path().join("nope")],
        device: device("kindle-paperwhite12").unwrap(),
        cut: Preset::Balanced.options(),
        title: "Book".into(),
        output: root.path().join("book.epub"),
    };
    assert!(export(&req, None, &mut |_| {}).is_err());
}
