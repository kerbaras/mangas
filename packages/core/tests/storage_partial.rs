//! A data-folder move killed midway leaves a marked partial copy that the next move may replace.
//! The kill is real: this test binary re-runs itself as the mover (`child_moves_until_killed`)
//! and the parent SIGKILLs it once the first file is copied.

mod move_support;

use std::fs;

use mangas_core::library::get_settings;
use mangas_core::storage::{
    move_data_dir, move_data_dir_with, partial_copy_at, remove_leftover_copy, usage, Folders,
    MOVE_MARKER,
};
use move_support::{folders, killed_move, library_with_two_chapters};

#[test]
fn a_killed_move_is_listed_refused_then_replaced_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("big");
    killed_move(&lib, tmp.path(), &to);
    assert!(to.join(MOVE_MARKER).exists());
    let left = usage(&lib, &f).unwrap().partial_move.expect("listed");
    assert_eq!(left.bytes, 100);
    assert_eq!(partial_copy_at(&to), Some(left));

    let err = move_data_dir(&lib, &f.data_dir, &to, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("partial copy"), "{err}");
    let moved = move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap();
    assert_eq!((moved.files, moved.bytes), (2, 200));
    let to = fs::canonicalize(&to).unwrap();
    assert!(!to.join(MOVE_MARKER).exists());
    assert_eq!(get_settings(&lib).unwrap().data_dir.as_deref(), to.to_str());
    let f = Folders { data_dir: to, ..f };
    assert_eq!(usage(&lib, &f).unwrap().partial_move, None);
}

#[test]
fn removing_the_partial_copy_keeps_the_rest_of_that_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("big");
    fs::create_dir_all(&to).unwrap();
    fs::write(to.join("notes.txt"), "mine").unwrap();
    killed_move(&lib, tmp.path(), &to);
    let removed = remove_leftover_copy(&lib, &f.data_dir).unwrap().unwrap();
    assert_eq!(removed.bytes, 100);
    assert!(!to.join("chapters").exists() && !to.join(MOVE_MARKER).exists());
    assert!(to.join("notes.txt").exists());
    assert!(f.data_dir.join("chapters/mangadex/a/2/001.jpg").exists());
    assert_eq!(usage(&lib, &f).unwrap().partial_move, None);
    assert_eq!(remove_leftover_copy(&lib, &f.data_dir).unwrap(), None);
    move_data_dir(&lib, &f.data_dir, &to, &mut |_| {}).unwrap();
}

#[test]
fn a_folder_with_unrelated_files_is_refused_even_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("other");
    fs::create_dir_all(to.join("chapters/x")).unwrap();
    fs::write(to.join("chapters/x/001.jpg"), "someone else's").unwrap();
    let err = move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("choose an empty folder"), "{err}");
    assert!(to.join("chapters/x/001.jpg").exists());
    assert_eq!(usage(&lib, &f).unwrap().partial_move, None);
}

#[test]
fn a_partial_copy_next_to_unrelated_files_is_refused_even_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let to = tmp.path().join("shared");
    fs::create_dir_all(&to).unwrap();
    fs::write(to.join("thesis.txt"), "mine").unwrap();
    killed_move(&lib, tmp.path(), &to);
    let err = move_data_dir_with(&lib, &f.data_dir, &to, true, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("thesis.txt"), "{err}");
    assert!(to.join("thesis.txt").exists() && to.join(MOVE_MARKER).exists());
    assert!(usage(&lib, &f).unwrap().partial_move.is_some());
    assert_ne!(get_settings(&lib).unwrap().data_dir.as_deref(), to.to_str());
}

#[test]
fn a_move_elsewhere_waits_until_the_partial_copy_is_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = library_with_two_chapters(&f);
    let first = tmp.path().join("first");
    killed_move(&lib, tmp.path(), &first);
    let second = tmp.path().join("second");
    let err = move_data_dir(&lib, &f.data_dir, &second, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("first"), "{err}");
    assert!(!second.exists(), "a refused move leaves no folder behind");
    assert!(usage(&lib, &f).unwrap().partial_move.is_some());
    remove_leftover_copy(&lib, &f.data_dir).unwrap().unwrap();
    move_data_dir(&lib, &f.data_dir, &second, &mut |_| {}).unwrap();
    assert!(!second.join(MOVE_MARKER).exists());
}
