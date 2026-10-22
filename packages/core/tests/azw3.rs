//! The AZW3 writer's output read back: by the `mobi` crate (an independent reader), and record by
//! record for the HD container and determinism.

use image::{DynamicImage, GrayImage, Luma};
use mangas_core::export::azw3;
use mangas_core::export::epub::{ChapterEntry, PageImage};
use mangas_core::export::{profile, Labels, Meta, PageSpool};

fn jpeg(w: u32, h: u32, noisy: bool) -> PageImage {
    let img = GrayImage::from_fn(w, h, |x, y| match noisy {
        true => Luma([((x * 7919 + y * 104_729) % 251) as u8]),
        false => Luma([(x / 8 + y / 8) as u8]),
    });
    let mut out = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(img)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .unwrap();
    PageImage {
        jpeg: out.into_inner(),
        width: w,
        height: h,
    }
}

fn book(rtl: bool) -> (Meta, Vec<PageImage>) {
    let pages = vec![
        jpeg(300, 400, false),
        jpeg(1264, 1680, true),
        jpeg(320, 400, false),
    ];
    let meta = Meta {
        id: "urn:uuid:azw3-test".into(),
        title: "Yotsuba&! ヨツバ".into(),
        authors: vec!["Azuma Kiyohiko".into()],
        language: "en".into(),
        series: None,
        right_to_left: rtl,
        chapters: vec![
            ChapterEntry {
                title: "Chapter 1".into(),
                first_page: 0,
            },
            ChapterEntry {
                title: "Chapter 2".into(),
                first_page: 2,
            },
        ],
        labels: Labels::default(),
        title_explicit: true,
        cover: Some(jpeg(200, 300, false)),
    };
    (meta, pages)
}

fn write(meta: &Meta, pages: &[PageImage]) -> (Vec<u8>, azw3::Report) {
    let spool = PageSpool::from_pages(pages.to_vec()).unwrap();
    let mut out = Vec::new();
    let report = azw3::write(
        &spool,
        meta,
        &profile("kindle-paperwhite12").unwrap(),
        &mut out,
    )
    .unwrap();
    (out, report)
}

/// The KF8 text from the crate's own record split: the PalmDOC header's text records, each
/// without its trailing byte. (`content_as_string_lossy` reads MOBI 6 fields KF8 reuses.)
fn text_of(m: &mobi::Mobi) -> String {
    let count = usize::from(m.metadata.palmdoc.record_count);
    let records = m.raw_records().into_iter().skip(1).take(count);
    let bytes: Vec<u8> = records
        .flat_map(|r| r.content[..r.content.len() - 1].to_vec())
        .collect();
    String::from_utf8(bytes).unwrap()
}

fn records(file: &[u8]) -> Vec<&[u8]> {
    let n = u16::from_be_bytes([file[76], file[77]]) as usize;
    let at =
        |i: usize| u32::from_be_bytes(file[78 + 8 * i..82 + 8 * i].try_into().unwrap()) as usize;
    (0..n)
        .map(|i| &file[at(i)..if i + 1 < n { at(i + 1) } else { file.len() }])
        .collect()
}

#[test]
fn mobi_crate_reads_title_author_and_images() {
    let (meta, pages) = book(false);
    let (file, _) = write(&meta, &pages);
    let m = mobi::Mobi::new(&file).unwrap();
    assert_eq!(m.title(), "Yotsuba&! ヨツバ");
    assert_eq!(m.author().as_deref(), Some("Azuma Kiyohiko"));
    // Cover, three pages, thumbnail; the crate also lists the index and HD records after them.
    let jpegs = m
        .image_records()
        .iter()
        .filter(|r| r.content.starts_with(&[0xFF, 0xD8, 0xFF]))
        .count();
    assert_eq!(jpegs, 5);
    let text = text_of(&m);
    assert_eq!(text.matches("<img src=\"kindle:embed:").count(), 4);
    assert!(text.contains("Yotsuba&amp;! &#x30E8;&#x30C4;&#x30D0;"));
}

#[test]
fn hd_container_keeps_every_jpeg_unchanged() {
    let (meta, pages) = book(false);
    let (file, report) = write(&meta, &pages);
    let recs = records(&file);
    let cres: Vec<&[u8]> = recs
        .iter()
        .filter(|r| r.starts_with(b"CRES"))
        .map(|r| &r[12..])
        .collect();
    let originals: Vec<&[u8]> = [meta.cover.as_ref().unwrap()]
        .into_iter()
        .chain(&pages)
        .map(|p| &p.jpeg[..])
        .collect();
    assert_eq!(cres, originals);
    assert_eq!(report.smaller_pages, [1]);
    assert!(!report.smaller_cover);
    let first_image = u32::from_be_bytes(recs[0][108..112].try_into().unwrap()) as usize;
    let standard = &recs[first_image..first_image + 5];
    assert!(standard
        .iter()
        .all(|r| r.starts_with(&[0xFF, 0xD8, 0xFF]) && r.len() <= 128 * 1024));
    assert_eq!(standard[0], originals[0]);
    assert_ne!(standard[2], originals[2]);
    assert!(recs.last().unwrap() == &[0xE9, 0x8E, 0x0D, 0x0A]);
}

#[test]
fn same_book_same_bytes_and_direction_only_changes_metadata() {
    let (meta, pages) = book(false);
    let (first, _) = write(&meta, &pages);
    assert_eq!(write(&meta, &pages).0, first);
    let (rtl_meta, _) = book(true);
    let (rtl, _) = write(&rtl_meta, &pages);
    let exth = |f: &[u8]| String::from_utf8_lossy(records(f)[0]).into_owned();
    assert!(exth(&first).contains("horizontal-lr") && exth(&first).contains("ltr"));
    assert!(exth(&rtl).contains("horizontal-rl") && exth(&rtl).contains("rtl"));
    assert_eq!(first.len(), rtl.len());
}

/// The QA fixture through the `mobi` crate:
/// `MANGAS_AZW3=out.azw3 cargo test -p mangas-core --test azw3 -- --ignored --nocapture`
#[test]
#[ignore]
fn reads_a_written_file() {
    let path = std::env::var("MANGAS_AZW3").expect("set MANGAS_AZW3 to an .azw3 file");
    let m = mobi::Mobi::from_path(&path).unwrap();
    let text = text_of(&m);
    let pages = text.matches("<img src=\"kindle:embed:").count();
    println!("title: {}", m.title());
    println!("author: {:?}", m.author());
    println!("language: {:?}", m.language());
    let jpegs = m
        .image_records()
        .iter()
        .filter(|r| r.content.starts_with(&[0xFF, 0xD8, 0xFF]))
        .count();
    println!("standard JPEG image records: {jpegs}");
    println!("xhtml pages (img tags): {pages}");
    println!("body text bytes: {}", text.len());
}
