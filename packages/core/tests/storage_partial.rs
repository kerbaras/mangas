//! A data-folder move killed midway leaves a marked partial copy that the next move may replace.
//! The kill is real: this test binary re-runs itself as the mover (`child_moves_until_killed`)
//! and the parent SIGKILLs it once the first file is copied.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use mangas_core::domain::{ChapterId, MangaId};
use mangas_core::library::{
    get_settings, mark_downloaded, open, set_queue_held_until, upsert_chapters, upsert_manga,
    Library,
};
use mangas_core::storage::{
    move_data_dir, move_data_dir_with, partial_copy_at, remove_leftover_copy, usage, Folders,
    MOVE_MARKER,
};

fn folders(root: &Path) -> Folders {
    Folders {
        data_dir: root.join("data"),
        database: root.join("app/mangas.db"),
        covers: root.join("app/covers"),
        logs: root.join("app/logs"),
    }
}

const ROOT_VAR: &str = "MANGAS_MOVE_TEST_ROOT";
const TO_VAR: &str = "MANGAS_MOVE_TEST_TO";

fn library_with_two_chapters(f: &Folders) -> Library {
    let data = &f.data_dir;
    fs::create_dir_all(f.database.parent().unwrap()).unwrap();
    let lib = open(&f.database).unwrap();
    let manga = mangas_core::domain::Manga {
        id: MangaId("a".into()),
        source: "mangadex".into(),
        title: "A".into(),
        ..Default::default()
    };
    upsert_manga(&lib, &manga).unwrap();
    let rows: Vec<_> = ["1", "2"]
        .iter()
        .map(|n| mangas_core::domain::Chapter {
            id: ChapterId(format!("a-{n}")),
            manga_id: MangaId("a".into()),
            language: "en".into(),
            number: (*n).into(),
            ..Default::default()
        })
        .collect();
    upsert_chapters(&lib, "mangadex", &MangaId("a".into()), "en", &rows).unwrap();
    for n in ["1", "2"] {
        let dir = data.join("chapters/mangadex/a").join(n);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("001.jpg"), vec![7u8; 100]).unwrap();
        mark_downloaded(&lib, "mangadex", &ChapterId(format!("a-{n}")), &dir).unwrap();
    }
    lib
}

#[test]
#[ignore = "run by killed_move"]
fn child_moves_until_killed() {
    let (Ok(root), Ok(to)) = (std::env::var(ROOT_VAR), std::env::var(TO_VAR)) else {
        return;
    };
    let f = folders(Path::new(&root));
    let lib = open(&f.database).unwrap();
    let ready = Path::new(&root).join("ready");
    let _ = move_data_dir(&lib, &f.data_dir, Path::new(&to), &mut |p| {
        if p.copied > 0 {
            fs::write(&ready, b"").unwrap();
            std::thread::sleep(Duration::from_secs(600));
        }
    });
}

/// Moves to `to` in a child process, SIGKILLs it after the first file is copied, then lets
/// the queue hold expire as a dead process's lease does.
fn killed_move(lib: &Library, root: &Path, to: &Path) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["child_moves_until_killed", "--exact", "--ignored"])
        .env(ROOT_VAR, root)
        .env(TO_VAR, to)
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !root.join("ready").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the move never copied a file"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    set_queue_held_until(lib, 0).unwrap();
}

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
    assert!(usage(&lib, &f).unwrap().partial_move.is_some());
    remove_leftover_copy(&lib, &f.data_dir).unwrap().unwrap();
    move_data_dir(&lib, &f.data_dir, &second, &mut |_| {}).unwrap();
    assert!(!second.join(MOVE_MARKER).exists());
}
