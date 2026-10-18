//! Backups carry the user's library state (#105): export → wipe → restore is row-for-row equal,
//! a backup made before the state existed still restores, and a restore into a library that
//! already has titles only adds.

use std::path::Path;

use mangas_core::backup::{export_to, import_from};
use mangas_core::domain::{Availability, Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    create_category, delete_category, get_settings, list_categories, list_entries, mark_downloaded,
    mark_exported, open, open_in_memory, set_auto_download, set_manga_categories,
    set_reading_status, set_settings, upsert_chapters, upsert_manga, Library, ReadingStatus,
    Settings,
};
use rusqlite::types::Value;
use rusqlite::Connection;

const KNOWN: &[&str] = &["mangadex", "tapas"];
const OLD_BACKUP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/backup/v1-main-dae16bc.json"
);

fn follow(lib: &Library, source: &str, id: &str) -> MangaId {
    let manga = Manga {
        id: MangaId(id.into()),
        source: source.into(),
        title: id.to_uppercase(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    manga.id
}

fn chapter(manga: &MangaId, id: &str, language: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        manga_id: manga.clone(),
        language: language.into(),
        number: id.trim_start_matches(char::is_alphabetic).into(),
        ..Default::default()
    }
}

/// A library using every piece of state the migrations up to 0008 added.
fn fill(lib: &Library, downloads: &Path) {
    let a = follow(lib, "mangadex", "a");
    let b = follow(lib, "mangadex", "b");
    let t = follow(lib, "tapas", "t");
    let fav = create_category(lib, "Favourites").unwrap();
    let gone = create_category(lib, "Gone").unwrap();
    let later = create_category(lib, "Later").unwrap();
    delete_category(lib, gone.id).unwrap();
    set_manga_categories(lib, "mangadex", &a, &[fav.id, later.id]).unwrap();
    set_manga_categories(lib, "tapas", &t, &[later.id]).unwrap();
    set_reading_status(lib, "mangadex", &b, ReadingStatus::OnHold).unwrap();
    set_reading_status(lib, "tapas", &t, ReadingStatus::Dropped).unwrap();
    set_auto_download(lib, "mangadex", &a, true).unwrap();

    let mut locked = chapter(&a, "a2", "en");
    locked.availability = Availability::External {
        url: Some("https://mangaplus.example/2".into()),
    };
    locked.group = Some("Group A, Group B".into());
    locked.volume = Some("1".into());
    locked.title = Some("Second".into());
    upsert_chapters(
        lib,
        "mangadex",
        &a,
        "en",
        &[chapter(&a, "a1", "en"), locked],
    )
    .unwrap();
    upsert_chapters(lib, "mangadex", &a, "es", &[chapter(&a, "a1es", "es")]).unwrap();
    // A later sync stamps the new chapter's first_seen_at.
    let newer = [chapter(&a, "a1", "en"), chapter(&a, "a3", "en")];
    upsert_chapters(lib, "mangadex", &a, "en", &newer).unwrap();
    upsert_chapters(lib, "tapas", &t, "en", &[chapter(&t, "t1", "en")]).unwrap();
    let dir = downloads.join("a1");
    std::fs::create_dir_all(&dir).unwrap();
    mark_downloaded(lib, "mangadex", &ChapterId("a1".into()), &dir).unwrap();
    assert_eq!(mark_exported(lib, &[dir.as_path()]).unwrap(), 1);
    set_settings(
        lib,
        &Settings {
            default_device: Some("kindle-scribe".into()),
            update_check_hours: Some(12),
            ..Default::default()
        },
    )
    .unwrap();
}

fn rows(conn: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |r| (0..width).map(|i| r.get(i)).collect())
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn columns_except(conn: &Connection, table: &str, skip: &[&str]) -> String {
    let names = rows(
        conn,
        &format!("SELECT name FROM pragma_table_info('{table}')"),
    );
    let names: Vec<String> = names
        .into_iter()
        .filter_map(|r| match &r[0] {
            Value::Text(n) if !skip.contains(&n.as_str()) => Some(n.clone()),
            _ => None,
        })
        .collect();
    names.join(", ")
}

/// Every library row a backup is responsible for. `SELECT *` on purpose: a column added by a
/// later migration fails the round trip until the backup carries it (or it is skipped here).
fn snapshot(db: &Path) -> Vec<Vec<Vec<Value>>> {
    let conn = Connection::open(db).unwrap();
    let chapter = columns_except(&conn, "chapter", &["local_path"]);
    let tables = rows(
        &conn,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    );
    let tables: Vec<_> = tables.into_iter().map(|r| r[0].clone()).collect();
    let known = [
        "category",
        "chapter",
        "download_jobs",
        "download_queue",
        "manga",
        "manga_category",
        "manga_language_sync",
        "setting",
    ];
    let known: Vec<_> = known.iter().map(|t| Value::Text(t.to_string())).collect();
    assert_eq!(tables, known, "new table: decide whether backups carry it");
    vec![
        rows(&conn, "SELECT * FROM manga ORDER BY rowid"),
        rows(
            &conn,
            "SELECT name, position FROM category ORDER BY position",
        ),
        rows(
            &conn,
            "SELECT mc.source, mc.manga_id, c.name FROM manga_category mc
             JOIN category c ON c.id = mc.category_id ORDER BY 1, 2, 3",
        ),
        rows(&conn, "SELECT * FROM manga_language_sync ORDER BY 1, 2, 3"),
        rows(
            &conn,
            &format!("SELECT {chapter} FROM chapter ORDER BY rowid"),
        ),
    ]
}

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
