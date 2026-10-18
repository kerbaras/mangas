//! Backups carry the user's library state (#105): export → wipe → restore is row-for-row equal,
//! a backup made before the state existed still restores, and a restore into a library that
//! already has titles only adds.

mod fixture;
mod rows;

use std::path::Path;

use fixture::{chapter, fill, follow};
use mangas_core::backup::{export_to, import_from};
use mangas_core::library::{
    create_category, get_settings, list_categories, list_entries, open, open_in_memory,
    set_manga_categories, set_reading_status, upsert_chapters, ReadingStatus,
};
use rows::{rows, snapshot};
use rusqlite::Connection;

const KNOWN: &[&str] = &["mangadex", "tapas"];
const OLD_BACKUP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/backup/v1-main-dae16bc.json"
);

#[test]
fn export_wipe_restore_is_row_for_row_equal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("library.db");
    let file = tmp.path().join("backup.json");
    let lib = open(&db).unwrap();
    fill(&lib, &tmp.path().join("chapters"));
    let before = snapshot(&db);
    let settings = get_settings(&lib).unwrap();
    export_to(&lib, &file, "test").unwrap();
    drop(lib);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }

    let lib = open(&db).unwrap();
    let report = import_from(&lib, &file, KNOWN).unwrap();
    assert_eq!((report.added, report.already), (3, 0));
    assert_eq!(snapshot(&db), before);
    assert_eq!(get_settings(&lib).unwrap(), settings);
    let conn = Connection::open(&db).unwrap();
    let paths = rows(
        &conn,
        "SELECT local_path FROM chapter WHERE local_path IS NOT NULL",
    );
    assert!(paths.is_empty(), "downloads aren't part of a backup");
}

#[test]
fn a_backup_from_before_the_library_state_still_restores() {
    let lib = open_in_memory().unwrap();
    let report = import_from(&lib, Path::new(OLD_BACKUP), KNOWN).unwrap();
    assert_eq!((report.added, report.settings), (2, true));
    assert!(list_categories(&lib).unwrap().is_empty());
    let entries = list_entries(&lib).unwrap();
    assert_eq!(entries.len(), 2);
    for e in &entries {
        assert_eq!(e.reading_status, ReadingStatus::Reading);
        assert!(e.categories.is_empty() && !e.auto_download);
        assert_eq!(
            e.added_at, "2026-10-18T08:19:08Z",
            "followed when bookmarked"
        );
    }
    let settings = get_settings(&lib).unwrap();
    assert_eq!(settings.default_device.as_deref(), Some("kindle-scribe"));
    assert_eq!(settings.update_check_hours, Some(12));
}

#[test]
fn restoring_into_a_library_with_titles_only_adds() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("backup.json");
    let source = open_in_memory().unwrap();
    fill(&source, &tmp.path().join("chapters"));
    export_to(&source, &file, "test").unwrap();

    let lib = open_in_memory().unwrap();
    let a = follow(&lib, "mangadex", "a");
    let mine = create_category(&lib, "Mine").unwrap();
    let fav = create_category(&lib, "FAVOURITES").unwrap();
    set_manga_categories(&lib, "mangadex", &a, &[mine.id]).unwrap();
    set_reading_status(&lib, "mangadex", &a, ReadingStatus::Completed).unwrap();
    upsert_chapters(&lib, "mangadex", &a, "en", &[chapter(&a, "a1", "en")]).unwrap();

    let report = import_from(&lib, &file, KNOWN).unwrap();
    assert_eq!((report.added, report.already), (2, 1));
    let names: Vec<_> = list_categories(&lib)
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(
        names,
        ["Mine", "FAVOURITES", "Later"],
        "matched ignoring case"
    );
    let entries = list_entries(&lib).unwrap();
    let entry = |id: &str| entries.iter().find(|e| e.manga.id.0 == id).unwrap();
    let later = list_categories(&lib).unwrap()[2].id;
    assert_eq!(entry("a").reading_status, ReadingStatus::Completed, "kept");
    assert_eq!(entry("a").categories, [mine.id, fav.id, later]);
    assert_eq!(entry("a").chapters.exported, 1, "the read mark comes back");
    assert_eq!(entry("b").reading_status, ReadingStatus::OnHold);
    assert_eq!(entry("t").categories, [later]);
}
