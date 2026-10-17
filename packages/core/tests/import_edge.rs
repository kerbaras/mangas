//! Import edge cases: sources inside the library folder, titles that share a slug, source edits.

use std::path::Path;

use image::{Rgba, RgbaImage};
use mangas_core::domain::MangaId;
use mangas_core::library::{
    import_paths, list_manga, open_in_memory, remove_manga, ImportRequest, LOCAL_SOURCE,
};

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

#[test]
fn a_series_keeps_its_id_after_the_slug_owner_is_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("data");
    for title in ["A B", "A! B"] {
        png(&tmp.path().join(title).join("Chapter 1"), "1.png");
    }
    let lib = open_in_memory().unwrap();
    let path = |t: &str| vec![tmp.path().join(t)];

    import_paths(&lib, &root, &request(path("A B"), None)).unwrap();
    let hashed = import_paths(&lib, &root, &request(path("A! B"), None)).unwrap();
    remove_manga(&lib, LOCAL_SOURCE, &MangaId("a-b".into())).unwrap();
    let again = import_paths(&lib, &root, &request(path("A! B"), None)).unwrap();

    assert_eq!(again.manga.id, hashed.manga.id);
    assert!(again.imported.is_empty());
    assert_eq!(list_manga(&lib).unwrap().len(), 1);
}

#[test]
fn imported_pages_are_copies_not_links_to_the_source() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("data");
    let source = tmp.path().join("Foo").join("Chapter 1");
    png(&source, "a.png");
    let lib = open_in_memory().unwrap();

    let report = import_paths(&lib, &root, &request(vec![tmp.path().join("Foo")], None)).unwrap();
    let page = report.imported[0].path.join("0001.png");
    let before = std::fs::read(&page).unwrap();
    std::fs::write(source.join("a.png"), b"edited").unwrap();
    std::fs::remove_dir_all(&source).unwrap();

    assert_eq!(std::fs::read(&page).unwrap(), before);
}
