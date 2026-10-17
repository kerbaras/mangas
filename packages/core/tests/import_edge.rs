//! Import edge cases: sources inside the library folder, titles that share a slug.

use std::path::Path;

use image::{Rgba, RgbaImage};
use mangas_core::library::{import_paths, list_manga, open_in_memory, ImportRequest};

fn png(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    RgbaImage::from_pixel(40, 60, Rgba([255, 255, 255, 255]))
        .save(dir.join(name))
        .unwrap();
}

fn request(paths: Vec<std::path::PathBuf>, title: Option<&str>) -> ImportRequest {
    ImportRequest {
        paths,
        title: title.map(str::to_string),
        language: "en".into(),
    }
}

#[test]
fn a_folder_already_in_the_library_layout_is_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("data");
    let inside = root.join("local").join("foo").join("1");
    png(&inside, "a.png");
    png(&inside, "b.png");
    let lib = open_in_memory().unwrap();

    let err = import_paths(&lib, &root, &request(vec![inside.clone()], Some("foo"))).unwrap_err();

    assert!(err.to_string().contains("no chapters"), "{err}");
    assert!(inside.join("a.png").is_file() && inside.join("b.png").is_file());
    assert!(list_manga(&lib).unwrap().is_empty());
}

#[test]
fn titles_with_the_same_slug_stay_separate_series() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("data");
    for title in ["A B", "A! B"] {
        png(&tmp.path().join(title).join("Chapter 1"), "1.png");
    }
    let lib = open_in_memory().unwrap();

    let first = import_paths(&lib, &root, &request(vec![tmp.path().join("A B")], None)).unwrap();
    let second = import_paths(&lib, &root, &request(vec![tmp.path().join("A! B")], None)).unwrap();
    let again = import_paths(&lib, &root, &request(vec![tmp.path().join("A! B")], None)).unwrap();

    assert_eq!(first.manga.id.0, "a-b");
    assert!(second.manga.id.0.starts_with("a-b-"), "{}", second.manga.id);
    assert_eq!(second.imported.len(), 1);
    assert_eq!(again.manga.id, second.manga.id);
    assert!(again.imported.is_empty());
    assert_eq!(list_manga(&lib).unwrap().len(), 2);
}
