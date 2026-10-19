//! #221: a 300-character series exports under a shortened file name in every format, and the
//! plan line names the file written (also when it is numbered or replaced).

use std::path::Path;
use std::process::Command;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::export::volume::MAX_STEM_BYTES;
use mangas_core::library;

const ID: &str = "long-1";

fn series() -> String {
    format!("{} Arc", "Ünending ".repeat(33).trim_end())
}

/// Follows a manga with a 300-character title and chapter "1" downloaded (one page).
fn library(home: &Path) {
    let db = home.join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let manga = Manga {
        id: MangaId(ID.into()),
        source: "mangadex".into(),
        title: series(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("c1".into()),
        manga_id: MangaId(ID.into()),
        number: "1".into(),
        language: "en".into(),
        ..Default::default()
    };
    library::upsert_chapters(
        &lib,
        "mangadex",
        &manga.id,
        "en",
        std::slice::from_ref(&chapter),
    )
    .unwrap();
    let dir = home.join("chapters/1");
    std::fs::create_dir_all(&dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("000.png")).unwrap();
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
}

/// Runs `mangas export <ID>` (one book) (plus `extra`) into `home/out`; returns the planned names.
fn export(home: &Path, extra: &[&str]) -> Vec<String> {
    let out = home.join("out");
    let run = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(["export", ID])
        .args(extra)
        .args(["-o", out.to_str().unwrap()])
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(run.status.success(), "{run:?}");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|l| l.split_once("  (").map(|(path, _)| path))
        .map(|p| {
            Path::new(p)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_300_character_title_exports_under_the_shortened_name_in_every_format() {
    assert_eq!(series().chars().count(), 300);
    let home = tempfile::tempdir().unwrap();
    library(home.path());
    let out = home.path().join("out");
    for format in ["epub", "cbz", "pdf"] {
        let planned = export(home.path(), &["--format", format]);
        let [name] = planned.as_slice() else {
            panic!("{planned:?}")
        };
        let stem = name.strip_suffix(&format!(".{format}")).unwrap();
        assert!(
            stem.len() <= MAX_STEM_BYTES && stem.ends_with("… ch 1"),
            "{name}"
        );
        assert!(out.join(name).is_file(), "{name} not in {:?}", files(&out));
    }
    assert_eq!(files(&out).len(), 3);
}

#[test]
fn the_shortened_name_is_numbered_next_to_a_taken_one_and_replaced_in_place() {
    let home = tempfile::tempdir().unwrap();
    library(home.path());
    let out = home.path().join("out");
    let first = export(home.path(), &[]);
    let second = export(home.path(), &[]);
    assert_eq!(second, [first[0].replace(".epub", " (2).epub")]);
    let replaced = export(home.path(), &["--replace"]);
    assert_eq!(replaced, first);
    let mut want = vec![first[0].clone(), second[0].clone()];
    want.sort();
    assert_eq!(files(&out), want);
}

#[test]
fn a_300_character_book_title_is_shortened_too() {
    let home = tempfile::tempdir().unwrap();
    library(home.path());
    let title = "T".repeat(300);
    let planned = export(home.path(), &["--title", &title]);
    assert_eq!(
        planned,
        [format!("{}….epub", "T".repeat(MAX_STEM_BYTES - 3))]
    );
    assert!(home.path().join("out").join(&planned[0]).is_file());
}
