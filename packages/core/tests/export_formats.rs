//! CBZ and PDF through the whole export pipeline, read back with independent parsers.

mod export_support;

use std::io::Read;

use export_support::{comic_info, run, tag_text};
use mangas_core::export::kindle::{export, ExportRequest};
use mangas_core::export::{profile, Format};
use mangas_core::smartcut::Preset;

#[test]
fn cbz_round_trips_pages_and_comic_info() {
    let (_root, stats) = run(Format::Cbz, "kobo-clara", "book.cbz");
    assert_eq!(stats.pages, 5);
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&stats.output).unwrap()).unwrap();
    let names: Vec<String> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
    let images: Vec<&String> = names.iter().filter(|n| n.ends_with(".jpg")).collect();
    assert_eq!(images.len(), 5);
    assert_eq!(images[0], "0001.jpg");
    let mut first = Vec::new();
    zip.by_name("0001.jpg")
        .unwrap()
        .read_to_end(&mut first)
        .unwrap();
    let decoded = image::load_from_memory(&first).unwrap();
    assert!(decoded.width() <= 1072 && decoded.height() <= 1448);

    let xml = comic_info(&stats.output);
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let text = |tag: &str| tag_text(&doc, tag);
    assert_eq!(text("Title").as_deref(), Some("Tom & Jerry"));
    assert_eq!(text("Series").as_deref(), Some("Tom & Jerry"));
    assert_eq!(text("Number"), None);
    assert_eq!(text("Volume"), None);
    assert_eq!(text("PageCount").as_deref(), Some("5"));
    assert_eq!(text("BlackAndWhite").as_deref(), Some("Yes"));
    let pages: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("Page"))
        .collect();
    assert_eq!(pages.len(), 5);
    assert_eq!(pages[0].attribute("Type"), Some("FrontCover"));
    let marks: Vec<_> = pages
        .iter()
        .filter_map(|p| Some((p.attribute("Image")?, p.attribute("Bookmark")?)))
        .collect();
    assert_eq!(marks, vec![("0", "Chapter 1"), ("2", "Chapter 2")]);
}

#[test]
fn pdf_has_one_page_per_image_sized_to_the_profile() {
    let (_root, stats) = run(Format::Pdf, "remarkable2", "book.pdf");
    let doc = lopdf::Document::load(&stats.output).unwrap();
    let pages = doc.get_pages();
    assert_eq!(pages.len(), stats.pages);
    assert_eq!(pages.len(), 5);
    let first = doc.get_object(pages[&1]).unwrap().as_dict().unwrap();
    let mbox = first.get(b"MediaBox").unwrap().as_array().unwrap();
    let num = |o: &lopdf::Object| o.as_float().unwrap();
    let (w, h) = (num(&mbox[2]), num(&mbox[3]));
    assert!((w - 1404.0 * 72.0 / 226.0).abs() < 0.1, "{w}");
    assert!((h - 1872.0 * 72.0 / 226.0).abs() < 0.1, "{h}");
    let images = doc
        .objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter(|s| s.dict.get(b"Subtype").and_then(|v| v.as_name()).ok() == Some(b"Image"))
        .count();
    assert_eq!(images, 5);
}

#[test]
fn colour_profiles_keep_colour_in_pdf() {
    let (_root, stats) = run(Format::Pdf, "tablet", "book.pdf");
    let doc = lopdf::Document::load(&stats.output).unwrap();
    let rgb = doc
        .objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter_map(|s| s.dict.get(b"ColorSpace").and_then(|v| v.as_name()).ok())
        .all(|cs| cs == b"DeviceRGB");
    assert!(rgb);
}

#[test]
fn every_format_keeps_an_existing_file() {
    for format in Format::ALL {
        let name = format!("book.{format}");
        let (root, stats) = run(format, "kindle-paperwhite12", &name);
        assert_eq!(stats.output, root.path().join(&name));
        let req = ExportRequest {
            chapter_dirs: vec![root.path().join("Chapter 1")],
            device: profile("kindle-paperwhite12").unwrap(),
            format,
            cut: Preset::Balanced.options(),
            page: Default::default(),
            title: "Again".into(),
            output: stats.output.clone(),
            replace: false,
            labels: Default::default(),
            title_explicit: false,
            meta: Default::default(),
        };
        let again = export(&req, None, &mut |_| {}).unwrap();
        assert_eq!(again.output, root.path().join(format!("book (2).{format}")));
    }
}
