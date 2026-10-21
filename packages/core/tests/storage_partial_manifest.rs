//! Reusing or removing a partial copy deletes only the files the killed move copied (listed in
//! its marker); files added there by hand stay.

mod move_support;

use std::fs;

use mangas_core::storage::{move_data_dir_with, remove_leftover_copy, MOVE_MARKER};
use move_support::{folders, killed_move, library_with_two_chapters};

#[test]
fn a_file_added_by_hand_survives_a_force_reuse() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("big");
    killed_move(&lib, tmp.path(), &to);
    let marker = fs::read_to_string(to.join(MOVE_MARKER)).unwrap();
    assert!(marker.contains("chapters/mangadex/a/2/001.jpg"), "{marker}");
    fs::create_dir_all(to.join("chapters/mine")).unwrap();
    fs::write(to.join("chapters/mine/notes.txt"), "mine").unwrap();
    fs::write(to.join("chapters/mangadex/a/scan.png"), "mine too").unwrap();

    let moved = move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap();
    assert_eq!((moved.files, moved.bytes), (2, 200));
    assert_eq!(
        fs::read(to.join("chapters/mine/notes.txt")).unwrap(),
        b"mine"
    );
    assert!(to.join("chapters/mangadex/a/scan.png").exists());
    assert!(to.join("chapters/mangadex/a/1/001.jpg").exists());
    assert!(!to.join(MOVE_MARKER).exists());
}

#[test]
fn removing_the_partial_copy_keeps_files_added_to_its_chapters() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("big");
    killed_move(&lib, tmp.path(), &to);
    fs::write(to.join("chapters/mangadex/a/1/notes.txt"), "mine").unwrap();

    let removed = remove_leftover_copy(&lib, &f.data_dir).unwrap().unwrap();
    assert_eq!(removed.bytes, 100, "only the copied file counts");
    assert!(to.join("chapters/mangadex/a/1/notes.txt").exists());
    assert!(!to.join("chapters/mangadex/a/1/001.jpg").exists());
    assert!(!to.join(MOVE_MARKER).exists());
}

#[test]
fn a_file_added_by_hand_where_the_move_copies_one_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("big");
    killed_move(&lib, tmp.path(), &to);
    // Downloaded since the crash, so the killed move never listed it.
    for root in [&f.data_dir, &to] {
        fs::create_dir_all(root.join("chapters/mangadex/a/3")).unwrap();
        fs::write(root.join("chapters/mangadex/a/3/001.jpg"), "3").unwrap();
    }
    let err = move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("files added by hand"), "{err}");
    assert!(
        err.to_string().contains("chapters/mangadex/a/3/001.jpg"),
        "{err}"
    );
    assert!(to.join(MOVE_MARKER).exists(), "nothing deleted");
    assert!(to.join("chapters/mangadex/a/3/001.jpg").exists());
}

#[test]
fn a_marker_without_a_list_counts_the_source_files_as_copied() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("old");
    fs::create_dir_all(to.join("chapters/mangadex/a/1")).unwrap();
    fs::write(to.join("chapters/mangadex/a/1/001.jpg"), [7u8; 50]).unwrap();
    fs::write(to.join("chapters/notes.txt"), "mine").unwrap();
    let from = fs::canonicalize(&f.data_dir).unwrap();
    fs::write(to.join(MOVE_MARKER), format!("{}\n", from.display())).unwrap();

    let moved = move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap();
    assert_eq!(moved.files, 2);
    assert!(to.join("chapters/notes.txt").exists());
    assert_eq!(
        fs::read(to.join("chapters/mangadex/a/1/001.jpg")).unwrap(),
        [7u8; 100]
    );
}

#[test]
fn a_listed_path_outside_the_copy_is_never_deleted() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("odd");
    fs::create_dir_all(to.join("chapters")).unwrap();
    fs::write(tmp.path().join("outside.txt"), "keep").unwrap();
    fs::write(to.join("top.txt"), "keep").unwrap();
    let copies = r#"["../outside.txt", "chapters/../../outside.txt", "top.txt", "chapters"]"#;
    let marker = format!(r#"{{"from": "/nowhere", "copies": {copies}}}"#);
    fs::write(to.join(MOVE_MARKER), marker).unwrap();

    let err = move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("top.txt"), "{err}");
    fs::remove_file(to.join("top.txt")).unwrap();
    move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap();
    assert!(tmp.path().join("outside.txt").exists());
}
