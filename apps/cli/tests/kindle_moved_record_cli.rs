//! #310: `mangas kindle` on a moved or copied chapter folder that resolves to a library chapter
//! (the match #307 names the book with) records that chapter as exported, and `--json` lists it
//! in `recorded`; a folder the library doesn't know marks nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;
use serde_json::{json, Value};

fn db(home: &Path) -> PathBuf {
    home.join("data/mangas/mangas.db")
}

/// A library following "Kaiju No. 8" (`mangadex/m-uuid`) with chapter 1 stored at
/// `<home>/data/mangas/chapters/mangadex/m-uuid/1`.
fn follow(home: &Path) {
    std::fs::create_dir_all(home.join("data/mangas")).unwrap();
    let lib = library::open(&db(home)).unwrap();
    let id = MangaId("m-uuid".into());
    let manga = Manga {
        id: id.clone(),
        source: "mangadex".into(),
        title: "Kaiju No. 8".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("c1".into()),
        manga_id: id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    library::upsert_chapters(&lib, "mangadex", &id, "en", std::slice::from_ref(&chapter)).unwrap();
    let stored = home.join("data/mangas/chapters/mangadex/m-uuid/1");
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &stored).unwrap();
}

/// When chapter 1 was last exported, straight from the database.
fn exported_at(home: &Path) -> Option<String> {
    let conn = rusqlite::Connection::open(db(home)).unwrap();
    conn.query_row("SELECT exported_at FROM chapter WHERE id = 'c1'", [], |r| {
        r.get(0)
    })
    .unwrap()
}

fn chapter(dir: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("001.png")).unwrap();
    dir
}

fn kindle_json(home: &Path, dir: &Path) -> Value {
    let out = home.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .arg("kindle")
        .arg(dir)
        .arg("-o")
        .arg(format!("{}/", out.display()))
        .arg("--json")
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(run.status.success(), "{run:?}");
    serde_json::from_slice(&run.stdout).unwrap_or_else(|e| panic!("{e}: {run:?}"))
}

#[test]
fn a_moved_chapter_folder_is_recorded_as_exported() {
    let home = tempfile::tempdir().unwrap();
    follow(home.path());
    let moved = chapter(home.path().join("moved/mangadex/m-uuid/1"));
    assert_eq!(exported_at(home.path()), None);
    let report = kindle_json(home.path(), &moved);
    let recorded = json!([{"path": moved, "manga": "Kaiju No. 8", "chapter": "1"}]);
    assert_eq!(report["recorded"], recorded, "{report}");
    assert!(exported_at(home.path()).is_some());
}

#[test]
fn a_folder_the_library_does_not_know_is_not_recorded() {
    let home = tempfile::tempdir().unwrap();
    follow(home.path());
    let other = chapter(home.path().join("elsewhere/Some Series/1"));
    let report = kindle_json(home.path(), &other);
    assert_eq!(report["recorded"], json!([]), "{report}");
    assert_eq!(exported_at(home.path()), None);
}
