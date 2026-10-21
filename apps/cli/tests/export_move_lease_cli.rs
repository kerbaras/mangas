//! An export and a data-folder move exclude each other across processes (#172): the export holds
//! the export lease shared while it reads chapters, the move takes it exclusively.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Manga, MangaId};
use mangas_core::library;
use mangas_core::ml::lease::{acquire_at, Purpose};
use mangas_core::storage::{hold_for_export, EXPORT_LEASE};

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn data(home: &Path) -> std::path::PathBuf {
    home.join("data/mangas")
}

#[test]
fn a_move_is_refused_while_an_export_reads_chapters() {
    let home = tempfile::tempdir().unwrap();
    let reading = hold_for_export(|| Ok::<_, mangas_core::Error>(data(home.path()))).unwrap();
    let to = home.path().join("elsewhere");
    let out = mangas(home.path(), &["storage", "move", to.to_str().unwrap()]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("an export or import is running; try again when it finishes"),
        "{err}"
    );
    drop(reading);
}

#[test]
fn an_export_is_refused_while_the_data_folder_moves_before_it_reads_a_chapter() {
    let home = tempfile::tempdir().unwrap();
    // The title must be followed on `--source` for the library lookup to pass before the lease
    // is checked; the lookup reads only the library, never a chapter.
    std::fs::create_dir_all(data(home.path())).unwrap();
    let lib = library::open(&data(home.path()).join("mangas.db")).unwrap();
    let manga = Manga {
        id: MangaId("any-series".into()),
        source: "mangadex".into(),
        title: "Any series".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    drop(lib);
    let moving = acquire_at(&data(home.path()), EXPORT_LEASE, Purpose::Move).unwrap();
    let out = mangas(
        home.path(),
        &[
            "export",
            "any-series",
            "--source",
            "mangadex",
            "-o",
            "/nonexistent",
        ],
    );
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    let kindle = mangas(home.path(), &["kindle", "/nonexistent/chapter"]);
    let err = String::from_utf8_lossy(&kindle.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    drop(moving);
}

#[test]
fn an_import_is_refused_while_the_data_folder_moves() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("Series/Chapter 1");
    std::fs::create_dir_all(&chapter).unwrap();
    std::fs::write(chapter.join("001.png"), b"not read").unwrap();
    let moving = acquire_at(&data(home.path()), EXPORT_LEASE, Purpose::Move).unwrap();
    let out = mangas(home.path(), &["import", chapter.to_str().unwrap()]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    assert!(!data(home.path()).join("downloads/local").exists());
    drop(moving);
}

#[test]
fn a_move_is_refused_while_an_import_copies_pages() {
    let home = tempfile::tempdir().unwrap();
    let importing = hold_for_export(|| Ok::<_, mangas_core::Error>(data(home.path()))).unwrap();
    let to = home.path().join("elsewhere");
    let out = mangas(home.path(), &["storage", "move", to.to_str().unwrap()]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("an export or import is running"), "{err}");
    drop(importing);
}

/// A followed title with one downloaded chapter; returns its chapter folder.
fn downloaded_chapter(home: &Path) -> std::path::PathBuf {
    use mangas_core::domain::{Chapter, ChapterId};
    use mangas_core::library::{mark_downloaded, open, upsert_chapters, upsert_manga};
    let lib = open(&data(home).join("mangas.db")).unwrap();
    let id = MangaId("a".into());
    let manga = Manga {
        id: id.clone(),
        source: "mangadex".into(),
        title: "A".into(),
        ..Default::default()
    };
    upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("a-1".into()),
        manga_id: id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    upsert_chapters(&lib, "mangadex", &id, "en", &[chapter]).unwrap();
    let dir = data(home).join("chapters/mangadex/a/1");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("001.jpg"), b"page").unwrap();
    mark_downloaded(&lib, "mangadex", &ChapterId("a-1".into()), &dir).unwrap();
    dir
}

// A child that another test thread is starting shares a dropped lock until it execs, so no test
// takes the lease again after dropping a move, or exclusively after dropping any.

/// `storage delete` during a move would leave the move's copies of the deleted chapters behind.
#[test]
fn a_download_delete_is_refused_while_the_data_folder_moves() {
    let home = tempfile::tempdir().unwrap();
    let dir = downloaded_chapter(home.path());
    let moving = acquire_at(&data(home.path()), EXPORT_LEASE, Purpose::Move).unwrap();
    let out = mangas(home.path(), &["storage", "delete", "a"]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    assert!(dir.join("001.jpg").exists(), "nothing deleted");
    drop(moving);
}

#[test]
fn a_move_is_refused_while_downloads_are_deleted() {
    let home = tempfile::tempdir().unwrap();
    let dir = downloaded_chapter(home.path());
    let deleting = hold_for_export(|| Ok::<_, mangas_core::Error>(data(home.path()))).unwrap();
    let to = home.path().join("elsewhere");
    let out = mangas(home.path(), &["storage", "move", to.to_str().unwrap()]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("an export or import is running"), "{err}");
    drop(deleting);

    // A delete shares the lease, so a child still sharing the dropped one doesn't refuse it.
    let out = mangas(home.path(), &["storage", "delete", "a"]);
    assert!(out.status.success(), "{out:?}");
    assert!(!dir.exists(), "deleted when no move runs");
}
