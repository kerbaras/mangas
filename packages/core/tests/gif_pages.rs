//! GIF pages: the shared format list, download names, Kindle export and local import.

use std::path::Path;

use image::{Rgba, RgbaImage};
use mangas_core::download::page_file_name;
use mangas_core::export::kindle::{chapter_files, device, export, ExportRequest};
use mangas_core::images::{has_page_extension, is_page_extension};
use mangas_core::library::{import_paths, open_in_memory, ImportRequest};
use mangas_core::smartcut::Preset;

/// `001.png`, `002.gif` (two frames, written with the `image` crate), `003.jpg`.
fn chapter_with_gif(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let white = RgbaImage::from_pixel(80, 120, Rgba([255, 255, 255, 255]));
    white.save(dir.join("001.png")).unwrap();
    let file = std::fs::File::create(dir.join("002.gif")).unwrap();
    let mut gif = image::codecs::gif::GifEncoder::new(file);
    let frames = [Rgba([0, 0, 0, 255]), Rgba([255, 255, 255, 255])]
        .map(|px| image::Frame::new(RgbaImage::from_pixel(80, 120, px)));
    gif.encode_frames(frames).unwrap();
    drop(gif);
    image::DynamicImage::ImageRgba8(white)
        .to_rgb8()
        .save(dir.join("003.jpg"))
        .unwrap();
}

#[test]
fn page_formats_are_the_decodable_ones() {
    for ext in ["jpg", "JPEG", "png", "webp", "gif", "GIF"] {
        assert!(is_page_extension(ext), "{ext}");
    }
    for ext in ["avif", "bmp", "txt", ""] {
        assert!(!is_page_extension(ext), "{ext}");
    }
    assert!(has_page_extension(Path::new("ch/0002.gif")));
    assert!(!has_page_extension(Path::new("ch/gif")));
}

#[test]
fn downloaded_gif_pages_keep_their_extension() {
    assert_eq!(page_file_name(2, "https://x/data/h/2-abc.gif"), "0002.gif");
}

#[test]
fn gif_decodes_as_its_first_frame() {
    let tmp = tempfile::tempdir().unwrap();
    chapter_with_gif(tmp.path());
    let img = mangas_core::export::kindle::decode(&tmp.path().join("002.gif")).unwrap();
    assert_eq!((img.width(), img.height()), (80, 120));
    assert_eq!(img.get_pixel(40, 60), &Rgba([0, 0, 0, 255]));
}

#[test]
fn export_keeps_gif_pages_and_counts_unsupported_files() {
    let tmp = tempfile::tempdir().unwrap();
    let ch = tmp.path().join("ch1");
    chapter_with_gif(&ch);
    std::fs::write(ch.join("004.avif"), b"not decodable here").unwrap();
    std::fs::write(ch.join(".DS_Store"), b"hidden").unwrap();

    let files = chapter_files(&ch).unwrap();
    assert_eq!(files.pages.len(), 3);
    assert_eq!(files.skipped, [ch.join("004.avif")]);

    let out = tmp.path().join("book.epub");
    let req = ExportRequest {
        format: mangas_core::export::Format::Epub,
        chapter_dirs: vec![ch],
        device: device("kindle-paperwhite12").unwrap(),
        cut: Preset::Balanced.options(),
        page: Default::default(),
        title: "Book".into(),
        output: out.clone(),
        replace: false,
        meta: Default::default(),
        labels: Default::default(),
        title_explicit: false,
    };
    let stats = export(&req, None, &mut |_| {}).unwrap();
    assert_eq!((stats.pages, stats.skipped), (3, 1));
    let zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let images = zip
        .file_names()
        .filter_map(Result::ok)
        .filter(|n| n.starts_with("OEBPS/images/"))
        .count();
    assert_eq!(images, 3);
}

#[test]
fn import_copies_gif_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("Series").join("Chapter 1");
    chapter_with_gif(&src);
    let data = tmp.path().join("data");
    let req = ImportRequest {
        paths: vec![src],
        title: Some("Series".into()),
        language: "en".into(),
    };
    let report = import_paths(&open_in_memory().unwrap(), &data, &req).unwrap();
    assert_eq!(report.pages(), 3);
    let dir = &report.imported[0].path;
    assert!(
        dir.join("0002.gif").is_file(),
        "{:?}",
        std::fs::read_dir(dir)
    );
}
