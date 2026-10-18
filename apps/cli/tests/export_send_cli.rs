//! `mangas export <id> --send` over several volumes, against a library built on disk and a
//! fake Kindle mount.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;

const ID: &str = "series-1";

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Follows a manga with chapters "1" and "2" downloaded; `pages[i]` pages in chapter i+1.
fn library_with(home: &Path, pages: [usize; 2]) {
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
    for (c, count) in chapters.iter().zip(pages) {
        let dir = home.join("chapters").join(&c.number);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..count {
            let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
            page.save(dir.join(format!("{i:03}.png"))).unwrap();
        }
        library::mark_downloaded(&lib, "mangadex", &c.id, &dir).unwrap();
    }
}

fn kindle(home: &Path) -> PathBuf {
    let root = home.join("Kindle");
    std::fs::create_dir_all(root.join("documents")).unwrap();
    root
}

fn export_send(home: &Path, via: &str, kindle: &Path) -> Output {
    let out_dir = home.join("out");
    let (out_dir, kindle) = (out_dir.to_str().unwrap(), kindle.to_str().unwrap());
    let args = [
        "export", ID, "--every", "1", "-o", out_dir, "--send", via, "--kindle", kindle,
    ];
    mangas(home, &args)
}

fn books(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|d| d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()))
        .into_iter()
        .flatten()
        .collect();
    names.sort();
    names
}

#[test]
fn every_volume_is_copied_to_the_kindle() {
    let home = tempfile::tempdir().unwrap();
    library_with(home.path(), [2, 2]);
    let kindle = kindle(home.path());
    let out = export_send(home.path(), "usb", &kindle);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(text(&out.stdout).matches("copied to").count(), 2, "{out:?}");
    let written = books(&home.path().join("out"));
    assert_eq!(written.len(), 2, "{written:?}");
    assert_eq!(books(&kindle.join("documents")), written);
}

#[test]
fn a_volume_is_sent_before_a_later_one_fails_to_export() {
    let home = tempfile::tempdir().unwrap();
    library_with(home.path(), [2, 0]);
    let kindle = kindle(home.path());
    let out = export_send(home.path(), "usb", &kindle);
    assert!(!out.status.success(), "{out:?}");
    assert_ne!(
        out.status.code(),
        Some(2),
        "an export error, not a send error"
    );
    let sent = books(&kindle.join("documents"));
    assert_eq!(sent, books(&home.path().join("out")));
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(text(&out.stdout).contains("copied to"), "{out:?}");
}

#[test]
fn unsent_volumes_are_each_named_and_exit_with_code_2() {
    let home = tempfile::tempdir().unwrap();
    library_with(home.path(), [2, 2]);
    let out = export_send(home.path(), "usb", &home.path().join("no-kindle"));
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(books(&home.path().join("out")).len(), 2);
    let err = text(&out.stderr);
    assert_eq!(err.matches("was written but not sent").count(), 2, "{err}");
    assert!(err.contains("2 book(s) were written but not sent"), "{err}");
}
