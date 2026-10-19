//! A preferences restore over an unreadable settings row refuses before writing anything.

use mangas_core::backup::{export_to, import_with, Preferences};
use mangas_core::domain::{Manga, MangaId};
use mangas_core::library::{
    list_manga, open, open_in_memory, set_settings, upsert_manga, Settings,
};

const KNOWN: &[&str] = &["mangadex"];
const BROKEN: &str = "{broken";

fn manga(id: &str) -> Manga {
    Manga {
        id: MangaId(id.into()),
        title: id.into(),
        source: "mangadex".into(),
        ..Default::default()
    }
}

fn raw_row(db: &std::path::Path) -> Vec<u8> {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row(
        "SELECT CAST(value AS BLOB) FROM setting WHERE key = 'app'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn a_preferences_restore_over_a_broken_row_refuses_before_following_titles() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("backup.json");
    let source = open_in_memory().unwrap();
    for id in ["a", "b", "new"] {
        upsert_manga(&source, &manga(id)).unwrap();
    }
    set_settings(&source, &Settings::default()).unwrap();
    export_to(&source, &file, "test").unwrap();

    let db = tmp.path().join("mangas.db");
    let lib = open(&db).unwrap();
    for id in ["a", "b", "c"] {
        upsert_manga(&lib, &manga(id)).unwrap();
    }
    set_settings(&lib, &Settings::default()).unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute("UPDATE setting SET value = ?1 WHERE key = 'app'", [BROKEN])
        .unwrap();

    let err = import_with(&lib, &file, KNOWN, Preferences::Backup).unwrap_err();
    assert!(err.to_string().contains("settings"), "{err}");
    assert_eq!(list_manga(&lib).unwrap().len(), 3, "no title followed");
    assert_eq!(raw_row(&db), BROKEN.as_bytes(), "the row is left as it was");

    let report = import_with(&lib, &file, KNOWN, Preferences::Keep).unwrap();
    assert_eq!(
        (report.added, report.kept_settings),
        (1, true),
        "keeping yours never reads them"
    );
    assert_eq!(raw_row(&db), BROKEN.as_bytes());
}
