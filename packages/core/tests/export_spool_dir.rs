//! The page spool goes in the export's own folder, not the system temp dir: an export works
//! when the temp dir can't hold (here: can't even create) the spooled book.

mod export_support;

use std::path::Path;

use export_support::chapter;
use mangas_core::export::kindle::{export, ExportRequest};
use mangas_core::export::{profile, Format};
use mangas_core::smartcut::Preset;

fn request(root: &Path, format: Format, replace: bool) -> ExportRequest {
    ExportRequest {
        chapter_dirs: vec![root.join("Chapter 1"), root.join("Chapter 2")],
        device: profile("kindle-paperwhite12").unwrap(),
        format,
        cut: Preset::Balanced.options(),
        page: Default::default(),
        title: "Book".into(),
        title_explicit: false,
        output: root.join(format!("book.{format}")),
        replace,
        split: None,
        meta: Default::default(),
        labels: Default::default(),
    }
}

#[test]
fn every_format_exports_with_no_usable_temp_dir() {
    let root = tempfile::tempdir().unwrap();
    chapter(root.path(), "Chapter 1", 3);
    chapter(root.path(), "Chapter 2", 2);
    std::env::set_var("TMPDIR", root.path().join("missing"));
    for format in [Format::Epub, Format::Cbz, Format::Pdf] {
        let stats = export(&request(root.path(), format, false), None, &mut |_| {});
        assert_eq!(stats.map(|s| s.pages).ok(), Some(5), "{format}");
    }
}
