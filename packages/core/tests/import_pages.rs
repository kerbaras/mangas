//! Import runs every page through the shared image check and names a lone CBZ after its file.

use std::io::Write;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::images::check_page;
use mangas_core::library::{import_paths, open_in_memory, ImportRequest, Skipped};

fn encoded(format: image::ImageFormat) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(30, 40, Rgba([0, 0, 0, 255])))
        .to_rgb8()
        .write_to(&mut out, format)
        .unwrap();
    out.into_inner()
}

fn image_dimensions_from_memory_ok(bytes: &[u8]) -> bool {
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .is_ok_and(|r| r.into_dimensions().is_ok())
}

/// 11 bytes with a `.jpg` name, as in the QA report of #89.
const FAKE: &[u8] = b"not a page!";

fn cbz(path: &Path, pages: &[(&str, &[u8])]) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    for (name, bytes) in pages {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn request(paths: Vec<PathBuf>) -> ImportRequest {
    ImportRequest {
        paths,
        title: None,
        language: "en".into(),
    }
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_fake_jpg_in_a_cbz_is_skipped_with_a_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let png = encoded(image::ImageFormat::Png);
    let path = tmp.path().join("Downloads").join("Berserk v01.cbz");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    cbz(
        &path,
        &[("01.png", &png), ("02.jpg", FAKE), ("03.png", &png)],
    );
    let lib = open_in_memory().unwrap();

    let report = import_paths(&lib, &tmp.path().join("data"), &request(vec![path])).unwrap();

    assert_eq!(report.manga.title, "Berserk v01");
    assert_eq!(report.imported.len(), 1);
    assert_eq!(report.imported[0].pages, 2);
    assert_eq!(files(&report.imported[0].path), ["0001.png", "0002.png"]);
    assert_eq!(
        report.skipped,
        [Skipped::new("Berserk v01.cbz/02.jpg", "not an image")]
    );
    assert!(
        report.summary().ends_with("; 1 skipped"),
        "{}",
        report.summary()
    );
}

#[test]
fn folder_pages_are_checked_and_named_for_their_real_format() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Solo").join("Chapter 1");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("1.png"), encoded(image::ImageFormat::Jpeg)).unwrap();
    std::fs::write(dir.join("2.jpg"), FAKE).unwrap();
    let jpeg = encoded(image::ImageFormat::Jpeg);
    std::fs::write(dir.join("3.jpg"), &jpeg[..jpeg.len() / 2]).unwrap();
    let lib = open_in_memory().unwrap();

    let report = import_paths(
        &lib,
        &tmp.path().join("data"),
        &request(vec![tmp.path().join("Solo")]),
    )
    .unwrap();

    assert_eq!(files(&report.imported[0].path), ["0001.jpg"]);
    assert_eq!(
        report.skipped,
        [
            Skipped::new("Chapter 1/2.jpg", "not an image"),
            Skipped::new("Chapter 1/3.jpg", "damaged JPG image"),
        ]
    );
}

#[test]
fn a_chapter_of_only_invalid_pages_is_not_imported() {
    let tmp = tempfile::tempdir().unwrap();
    let series = tmp.path().join("Mixed");
    std::fs::create_dir_all(&series).unwrap();
    cbz(&series.join("c1.cbz"), &[("a.jpg", FAKE)]);
    cbz(
        &series.join("c2.cbz"),
        &[("a.png", &encoded(image::ImageFormat::Png))],
    );
    let lib = open_in_memory().unwrap();

    let report = import_paths(&lib, &tmp.path().join("data"), &request(vec![series])).unwrap();

    assert_eq!(report.manga.title, "Mixed");
    assert_eq!(report.imported.len(), 1);
    assert_eq!(
        report.skipped,
        [
            Skipped::new("c1.cbz/a.jpg", "not an image"),
            Skipped::new("c1.cbz", "no valid images"),
        ]
    );
}

#[test]
fn several_cbz_files_still_take_the_parent_folder_name() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = tmp.path().join("Vagabond");
    std::fs::create_dir_all(&parent).unwrap();
    let png = encoded(image::ImageFormat::Png);
    let paths = vec![parent.join("v01.cbz"), parent.join("v02.cbz")];
    for p in &paths {
        cbz(p, &[("1.png", &png)]);
    }
    let lib = open_in_memory().unwrap();

    let report = import_paths(&lib, &tmp.path().join("data"), &request(paths)).unwrap();

    assert_eq!(report.manga.title, "Vagabond");
}

#[test]
fn check_page_names_what_is_wrong() {
    let png = encoded(image::ImageFormat::Png);
    let jpeg = encoded(image::ImageFormat::Jpeg);
    assert_eq!(check_page(&png), Ok("png"));
    assert_eq!(check_page(&jpeg), Ok("jpg"));
    assert_eq!(check_page(FAKE), Err("not an image".to_string()));
    assert_eq!(check_page(&png[..20]), Err("damaged PNG image".to_string()));
    // Header intact (the size reads fine), pixel data cut off: only a full decode catches it.
    let cut_pixels = &png[..png.len() - 16];
    assert!(image_dimensions_from_memory_ok(cut_pixels));
    assert_eq!(check_page(cut_pixels), Err("damaged PNG image".to_string()));
    assert_eq!(
        check_page(&jpeg[..jpeg.len() - 10]),
        Err("damaged JPG image".to_string())
    );
    let avif = b"\0\0\0\x1cftypavif\0\0\0\0avifmif1miaf";
    assert_eq!(
        check_page(avif),
        Err("AVIF images aren't supported".to_string())
    );
}
