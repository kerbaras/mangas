//! An AZW3 book takes its metadata from the same request as the EPUB: the authors when known
//! (no creator at all when not, never "mangas") and the reading direction.

mod export_support;

use std::path::Path;

use export_support::chapter;
use mangas_core::domain::ReadingDirection;
use mangas_core::export::kindle::{export, BookMeta, ExportRequest};
use mangas_core::export::{profile, Format};
use mangas_core::smartcut::Preset;

fn azw3(root: &Path, meta: BookMeta) -> Vec<u8> {
    chapter(root, "Chapter 1", 2);
    let req = ExportRequest {
        split: None,
        chapter_dirs: vec![root.join("Chapter 1")],
        device: profile("kindle-paperwhite12").unwrap(),
        format: Format::Azw3,
        cut: Preset::Balanced.options(),
        page: Default::default(),
        title: "Book".into(),
        title_explicit: false,
        output: root.join("book.azw3"),
        replace: false,
        meta,
        labels: Default::default(),
    };
    let stats = export(&req, None, &mut |_| {}).unwrap();
    std::fs::read(stats.output).unwrap()
}

fn has(file: &[u8], needle: &[u8]) -> bool {
    file.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn an_unknown_author_leaves_the_creator_out() {
    let root = tempfile::tempdir().unwrap();
    let file = azw3(root.path(), BookMeta::default());
    let m = mobi::Mobi::new(&file).unwrap();
    assert_eq!(m.author(), None);
    // EXTH 535 (the build tool) says "mangas"; a creator (EXTH 100) never does.
    let creator = [&100u32.to_be_bytes()[..], &14u32.to_be_bytes(), b"mangas"].concat();
    assert!(!has(&file, &creator));
    assert!(has(
        &file,
        &[&535u32.to_be_bytes()[..], &14u32.to_be_bytes(), b"mangas"].concat()
    ));
}

#[test]
fn authors_language_and_direction_come_from_the_request() {
    let root = tempfile::tempdir().unwrap();
    let meta = BookMeta {
        authors: vec!["Azuma Kiyohiko".into()],
        language: Some("ja".into()),
        direction: ReadingDirection::Rtl,
        ..Default::default()
    };
    let file = azw3(root.path(), meta);
    let m = mobi::Mobi::new(&file).unwrap();
    assert_eq!(m.author().as_deref(), Some("Azuma Kiyohiko"));
    assert!(has(&file, b"horizontal-rl"));
}
