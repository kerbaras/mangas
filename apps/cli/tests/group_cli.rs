//! `mangas group` and `mangas chapters --group` on a fixture library (#160), no network.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{self, chapter_group, upsert_chapters, upsert_manga};

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

fn db(home: &Path) -> std::path::PathBuf {
    home.join("data/mangas/mangas.db")
}

/// Chapter 1 from two groups and the site mirror, chapter 2 from one group.
fn fixture(home: &Path) {
    std::fs::create_dir_all(home.join("data/mangas")).unwrap();
    let lib = library::open(&db(home)).unwrap();
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
        ..Default::default()
    };
    let list = [
        chapter("u:1", "1", "K Manga"),
        chapter("u:2", "1", "Official"),
        chapter("c:3", "1", "MangaDot mirror"),
        chapter("u:4", "2", "K Manga"),
    ];
    upsert_chapters(&lib, "mangadot", &id, "en", &list).unwrap();
}

const OFFLINE: [&str; 6] = [
    "chapters",
    "2098",
    "--source",
    "mangadot",
    "--offline",
    "--lang",
];

#[test]
fn chapters_show_the_group_and_filter_by_it() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());
    let all = stdout(&mangas(home.path(), &[&OFFLINE[..], &["en"]].concat()));
    assert!(all.lines().next().unwrap().contains("group"), "{all}");
    assert!(
        all.contains("Official") && all.contains("MangaDot mirror"),
        "{all}"
    );

    let args = [&OFFLINE[..], &["en", "--group", "K Manga"]].concat();
    let narrowed = stdout(&mangas(home.path(), &args));
    let rows: Vec<&str> = narrowed.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(rows.len(), 2, "{narrowed}");
    assert!(rows.iter().all(|r| r.contains("K Manga")), "{narrowed}");

    let args = [&OFFLINE[..], &["en", "--group", "Nobody"]].concat();
    let unknown = mangas(home.path(), &args);
    assert!(!unknown.status.success(), "{unknown:?}");
    let err = String::from_utf8_lossy(&unknown.stderr);
    assert!(err.contains("no chapters of group \"Nobody\""), "{err}");
    assert!(err.contains("--source mangadot"), "{err}");
}

#[test]
fn group_saves_lists_and_clears_the_choice() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());
    let id = MangaId("2098".into());
    let saved = || chapter_group(&library::open(&db(home.path())).unwrap(), "mangadot", &id);
    let group = |extra: &[&str]| {
        let args = [&["group", "2098", "--source", "mangadot"][..], extra].concat();
        mangas(home.path(), &args)
    };

    let listed = stdout(&group(&[]));
    assert!(listed.contains("    2  K Manga"), "{listed}");
    assert!(listed.contains("Every group shown"), "{listed}");

    stdout(&group(&["Official"]));
    assert_eq!(saved().unwrap().as_deref(), Some("Official"));
    assert!(stdout(&group(&[])).contains("*     1  Official"));

    let unknown = group(&["Nobody"]);
    assert!(!unknown.status.success());
    assert_eq!(saved().unwrap().as_deref(), Some("Official"));

    stdout(&group(&["--clear"]));
    assert_eq!(saved().unwrap(), None);
}
