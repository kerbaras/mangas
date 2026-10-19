//! `mangas export --replace` over several books planned under one file name.

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

fn export_replace(home: &Path) {
    let out = home.join("out");
    let args = [
        "export", ID, "--every", "1", "--name", "Book", "--format", "epub",
    ];
    let status = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .args(["--replace", "-o", out.to_str().unwrap()])
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(status.status.success(), "{status:?}");
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
    export_replace(home.path());
    let out = home.path().join("out");
    assert_eq!(books(&out), ["Book (2).epub", "Book.epub"]);
    export_replace(home.path());
    assert_eq!(
        books(&out),
        ["Book (2).epub", "Book.epub"],
        "replaced, no (3)"
    );
}
