//! Fixed-layout EPUB structure.

use std::io::Read;

use image::{ExtendedColorType, RgbImage};
use mangas_core::export::epub::{self, Book, ChapterEntry, PageImage};
use mangas_core::export::kindle::process::encode_jpeg;

fn page(w: u32, h: u32) -> PageImage {
    let img = RgbImage::from_pixel(w, h, image::Rgb([200, 200, 200]));
    let jpeg = encode_jpeg(img.as_raw(), w, h, ExtendedColorType::Rgb8, 80).expect("jpeg");
    PageImage {
        jpeg,
        width: w,
        height: h,
    }
}

#[test]
fn writes_valid_fixed_layout_epub() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("sub/out.epub");
    let book = Book {
        id: "1234".into(),
        title: "Test <Book>".into(),
        author: "mangas".into(),
        language: "en".into(),
        right_to_left: false,
        device_width: 1264,
        device_height: 1680,
        chapters: vec![
            ChapterEntry {
                title: "Ch 1".into(),
                first_page: 0,
            },
            ChapterEntry {
                title: "Ch 2".into(),
                first_page: 2,
            },
        ],
        pages: vec![page(1264, 1680), page(1264, 1680), page(1264, 1680)],
    };
    epub::write(&book, &out).expect("write");

    let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let first = zip.by_index(0).unwrap();
    assert_eq!(first.name(), "mimetype");
    assert_eq!(first.compression(), zip::CompressionMethod::Stored);
    drop(first);
    let mut mime = String::new();
    zip.by_name("mimetype")
        .unwrap()
        .read_to_string(&mut mime)
        .unwrap();
    assert_eq!(mime, "application/epub+zip");

    let mut container = String::new();
    zip.by_name("META-INF/container.xml")
        .unwrap()
        .read_to_string(&mut container)
        .unwrap();
    assert!(container.contains("full-path=\"OEBPS/content.opf\""));

    let mut opf = String::new();
    zip.by_name("OEBPS/content.opf")
        .unwrap()
        .read_to_string(&mut opf)
        .unwrap();
    assert!(opf.contains("<meta property=\"rendition:layout\">pre-paginated</meta>"));
    assert!(opf.contains("<meta name=\"original-resolution\" content=\"1264x1680\"/>"));
    assert!(opf.contains("<meta name=\"book-type\" content=\"comic\"/>"));
    assert!(opf.contains("<dc:title>Test &lt;Book&gt;</dc:title>"));
    assert_eq!(opf.matches("<itemref idref=").count(), 3);
    assert!(opf.contains("properties=\"nav\""));

    let mut nav = String::new();
    zip.by_name("OEBPS/nav.xhtml")
        .unwrap()
        .read_to_string(&mut nav)
        .unwrap();
    assert!(nav.contains("text/page-00002.xhtml"));

    let mut xhtml = String::new();
    zip.by_name("OEBPS/text/page-00000.xhtml")
        .unwrap()
        .read_to_string(&mut xhtml)
        .unwrap();
    assert!(xhtml.contains("<img src=\"../images/page-00000.jpg\""));
    assert!(xhtml.contains("width=1264, height=1680"));
    assert!(zip.by_name("OEBPS/images/page-00002.jpg").is_ok());
}

#[test]
fn rejects_empty_book() {
    let dir = tempfile::tempdir().unwrap();
    let book = Book {
        id: "x".into(),
        title: "t".into(),
        author: "a".into(),
        language: "en".into(),
        right_to_left: false,
        device_width: 1,
        device_height: 1,
        chapters: vec![],
        pages: vec![],
    };
    assert!(epub::write(&book, &dir.path().join("e.epub")).is_err());
}
