//! Folders and CBZ files on disk -> `local` manga in the library -> Kindle EPUB.

use std::io::Write;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{device, export, ExportRequest};
use mangas_core::library::{
    import_paths, is_local, list_chapters, list_manga, open_in_memory, ImportRequest, LOCAL_SOURCE,
};
use mangas_core::smartcut::Preset;

fn png(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    RgbaImage::from_pixel(80, 120, Rgba([255, 255, 255, 255]))
        .save(dir.join(name))
        .unwrap();
}

fn png_bytes() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    RgbaImage::from_pixel(80, 120, Rgba([0, 0, 0, 255]))
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

fn cbz(path: &Path, pages: &[&str]) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    for name in pages {
        zip.start_file(*name, opts).unwrap();
        zip.write_all(&png_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

fn request(paths: Vec<PathBuf>, title: Option<&str>) -> ImportRequest {
    ImportRequest {
        paths,
        title: title.map(str::to_string),
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

/// `Berserk/` with two chapter folders, one CBZ, an empty folder and a stray text file.
fn series(root: &Path) -> PathBuf {
    let dir = root.join("Berserk");
    for p in ["10.png", "2.png", "1.png"] {
        png(&dir.join("Chapter 1"), p);
    }
    png(&dir.join("Chapter 2"), "a.png");
    std::fs::create_dir_all(dir.join("Extras")).unwrap();
    std::fs::write(dir.join("notes.txt"), b"x").unwrap();
    cbz(
        &dir.join("Berserk c003.cbz"),
        &["__MACOSX/._p1.png", "p/p10.png", "p/p2.png"],
    );
    dir
}

#[test]
fn imports_a_series_folder_and_exports_it() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    let lib = open_in_memory().unwrap();
    let report = import_paths(&lib, &data, &request(vec![series(tmp.path())], None)).unwrap();

    assert_eq!(report.manga.title, "Berserk");
    assert_eq!(report.manga.id.0, "berserk");
    assert!(is_local(&report.manga.source));
    let numbers: Vec<&str> = report.imported.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["1", "2", "3"]);
    assert_eq!(report.pages(), 6);
    let mut skipped: Vec<(&str, &str)> = report
        .skipped
        .iter()
        .map(|s| (s.name.as_str(), s.reason.as_str()))
        .collect();
    skipped.sort();
    assert_eq!(
        skipped,
        [
            ("Extras", "no images"),
            ("notes.txt", "not a chapter folder or .cbz file")
        ]
    );

    let ch1 = data.join("local/berserk/1");
    assert_eq!(report.imported[0].path, ch1);
    assert_eq!(files(&ch1), ["0001.png", "0002.png", "0003.png"]);
    assert_eq!(
        files(&data.join("local/berserk/3")),
        ["0001.png", "0002.png"]
    );
    assert!(!data.join("local/berserk/Extras").exists());

    let stored = list_chapters(&lib, LOCAL_SOURCE, &report.manga.id, Some("en")).unwrap();
    assert_eq!(stored.len(), 3);
    assert!(stored.iter().all(|c| c.is_downloaded()));
    assert_eq!(list_manga(&lib).unwrap().len(), 1);

    let out = tmp.path().join("berserk.epub");
    let req = ExportRequest {
        chapter_dirs: stored
            .iter()
            .map(|c| PathBuf::from(c.local_path.clone().unwrap()))
            .collect(),
        device: device("kindle-paperwhite12").unwrap(),
        cut: Preset::Balanced.options(),
        title: report.manga.title.clone(),
        output: out.clone(),
        meta: Default::default(),
    };
    let stats = export(&req, None, &mut |_| {}).unwrap();
    assert_eq!(stats.pages, 6);
    assert!(out.is_file());
}

#[test]
fn reimport_only_adds_new_chapters() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    let lib = open_in_memory().unwrap();
    let dir = series(tmp.path());
    import_paths(&lib, &data, &request(vec![dir.clone()], None)).unwrap();
    png(&dir.join("Chapter 4"), "1.png");
    let again = import_paths(&lib, &data, &request(vec![dir], None)).unwrap();
    let numbers: Vec<&str> = again.imported.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["4"]);
    assert_eq!(
        again
            .skipped
            .iter()
            .filter(|s| s.reason == "already imported")
            .count(),
        3
    );
    let stored = list_chapters(&lib, LOCAL_SOURCE, &again.manga.id, None).unwrap();
    assert_eq!(stored.len(), 4);
}

#[test]
fn cbz_files_and_image_folders_are_chapters_of_their_parent() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = tmp.path().join("Tower of God");
    std::fs::create_dir_all(&parent).unwrap();
    cbz(&parent.join("ch 1.cbz"), &["1.png"]);
    png(&parent.join("ch 2"), "1.png");
    let lib = open_in_memory().unwrap();
    let paths = vec![parent.join("ch 2"), parent.join("ch 1.cbz")];
    let report = import_paths(&lib, &tmp.path().join("data"), &request(paths, None)).unwrap();
    assert_eq!(report.manga.title, "Tower of God");
    let numbers: Vec<&str> = report.imported.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["1", "2"]);

    let single = vec![parent.join("ch 2")];
    let titled = import_paths(
        &lib,
        &tmp.path().join("data"),
        &request(single, Some("ToG")),
    )
    .unwrap();
    assert_eq!(
        (titled.manga.id.0.as_str(), titled.imported.len()),
        ("tog", 1)
    );
}

#[test]
fn nothing_importable_is_an_error_and_stores_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("Empty/Chapter 1")).unwrap();
    std::fs::write(tmp.path().join("broken.cbz"), b"not a zip").unwrap();
    let lib = open_in_memory().unwrap();
    let data = tmp.path().join("data");
    assert!(import_paths(&lib, &data, &request(vec![tmp.path().join("Empty")], None)).is_err());
    assert!(import_paths(
        &lib,
        &data,
        &request(vec![tmp.path().join("broken.cbz")], None)
    )
    .is_err());
    assert!(import_paths(
        &lib,
        &data,
        &request(vec![tmp.path().join("missing")], None)
    )
    .is_err());
    assert!(list_manga(&lib).unwrap().is_empty());
}
