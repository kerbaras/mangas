//! `mangas export` with and without `--replace` over several books planned under one file name:
//! the plan lines name the files written.

use std::path::Path;
use std::process::Command;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;

const ID: &str = "series-1";

/// Follows a manga with chapters "1" and "2" downloaded, one page each.
fn library(home: &Path) {
    let db = home.join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let manga = Manga {
        id: MangaId(ID.into()),
        source: "mangadex".into(),
        title: "Series".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = |n: &str| Chapter {
        id: ChapterId(format!("c{n}")),
        manga_id: MangaId(ID.into()),
        number: n.into(),
        language: "en".into(),
        ..Default::default()
    };
    let chapters = [chapter("1"), chapter("2")];
    library::upsert_chapters(&lib, "mangadex", &manga.id, "en", &chapters).unwrap();
    for c in &chapters {
        let dir = home.join("chapters").join(&c.number);
        std::fs::create_dir_all(&dir).unwrap();
        let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
        page.save(dir.join("000.png")).unwrap();
        library::mark_downloaded(&lib, "mangadex", &c.id, &dir).unwrap();
    }
}

/// Runs `mangas export --every 1 --name Book` (plus `extra`) into `home/out`; returns stdout.
fn export(home: &Path, extra: &[&str]) -> String {
    let out = home.join("out");
    let args = [
        "export", ID, "--every", "1", "--name", "Book", "--format", "epub",
    ];
    let run = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .args(extra)
        .args(["-o", out.to_str().unwrap()])
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(run.status.success(), "{run:?}");
    String::from_utf8_lossy(&run.stdout).into_owned()
}

fn export_replace(home: &Path) -> String {
    export(home, &["--replace"])
}

/// The file names the plan lines (`<path>  (N chapters, a–b)`) announce.
fn planned(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|l| l.split_once("  (").map(|(path, _)| path))
        .map(|path| {
            Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn books(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn replace_keeps_one_file_per_book_and_writes_over_them_on_the_next_run() {
    let home = tempfile::tempdir().unwrap();
    library(home.path());
    let shown = export_replace(home.path());
    assert_eq!(planned(&shown), ["Book.epub", "Book (2).epub"], "{shown}");
    let out = home.path().join("out");
    assert_eq!(books(&out), ["Book (2).epub", "Book.epub"]);
    let shown = export_replace(home.path());
    assert_eq!(planned(&shown), ["Book.epub", "Book (2).epub"], "{shown}");
    assert_eq!(
        books(&out),
        ["Book (2).epub", "Book.epub"],
        "replaced, no (3)"
    );
}

#[test]
fn the_plan_names_the_files_a_default_export_writes() {
    let home = tempfile::tempdir().unwrap();
    library(home.path());
    let out = home.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("Book.epub"), "old").unwrap();
    let dry = export(home.path(), &["--dry-run"]);
    assert_eq!(planned(&dry), ["Book (2).epub", "Book (3).epub"], "{dry}");
    let shown = export(home.path(), &[]);
    assert_eq!(planned(&shown), planned(&dry), "{shown}");
    assert_eq!(books(&out), ["Book (2).epub", "Book (3).epub", "Book.epub"]);
    assert_eq!(std::fs::read(out.join("Book.epub")).unwrap(), b"old");
}
