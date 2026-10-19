//! #195: `mangas chapters --json` on a fixture library, no network.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{self, upsert_chapters, upsert_manga};
use serde_json::Value;

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Chapters 1 (two groups), 2 (downloaded) and 3 (locked).
fn fixture(home: &Path) {
    std::fs::create_dir_all(home.join("data/mangas")).unwrap();
    let lib = library::open(&home.join("data/mangas/mangas.db")).unwrap();
    let id = MangaId("2098".into());
    let manga = Manga {
        id: id.clone(),
        source: "mangadot".into(),
        title: "Knight".into(),
        ..Default::default()
    };
    upsert_manga(&lib, &manga).unwrap();
    let chapter = |cid: &str, number: &str, group: &str| Chapter {
        id: ChapterId(cid.into()),
        manga_id: id.clone(),
        language: "en".into(),
        number: number.into(),
        group: Some(group.into()),
        title: Some(format!("Chapter {number}")),
        ..Default::default()
    };
    let downloaded = chapter("u:3", "2", "K Manga");
    let mut locked = chapter("u:4", "3", "K Manga");
    locked.availability = mangas_core::domain::Availability::Locked;
    let list = [
        chapter("u:1", "1", "K Manga"),
        chapter("u:2", "1", "Official"),
        downloaded,
        locked,
    ];
    upsert_chapters(&lib, "mangadot", &id, "en", &list).unwrap();
    let folder = Path::new("/books/knight/2");
    library::mark_downloaded(&lib, "mangadot", &ChapterId("u:3".into()), folder).unwrap();
}

fn chapters(home: &Path, extra: &[&str]) -> String {
    let base = ["chapters", "2098", "--source", "mangadot", "--offline"];
    stdout(&mangas(home, &[&base[..], extra].concat()))
}

#[test]
fn json_rows_match_the_table_one_to_one_and_are_stable() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());
    let table = chapters(home.path(), &[]);
    let json = chapters(home.path(), &["--json"]);
    assert_eq!(json, chapters(home.path(), &["--json"]), "not byte-stable");
    let rows: Vec<Value> = serde_json::from_str(&json).unwrap();
    let lines: Vec<&str> = table.lines().skip(1).collect();
    assert_eq!(rows.len(), lines.len(), "{table}\n{json}");
    for (row, line) in rows.iter().zip(&lines) {
        let cells: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(row["number"], cells[0], "{line}");
        assert!(line.contains(row["id"].as_str().unwrap()), "{line}");
    }
    assert_eq!(rows[2]["local_path"], "/books/knight/2");
    assert!(lines[2].contains("on disk"), "{table}");
    assert_eq!(rows[3]["availability"]["kind"], "locked");
    assert!(rows[0]["exported_at"].is_null());
}

#[test]
fn json_respects_the_group() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());
    let json = chapters(home.path(), &["--json", "--group", "Official"]);
    let rows: Vec<Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(rows.len(), 1, "{json}");
    assert_eq!(rows[0]["group"], "Official");
}
