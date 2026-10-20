//! #303: `mangas export` into a folder we may write to but not list (mode 0300, a drop box): the
//! dry-run plans the names the real run writes, and a plain export still goes in (1411be1).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
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

/// Makes `home/out` holding `files` unlistable (0300). False when the mode doesn't stop listing
/// (root), so the test has nothing to check.
fn drop_box(home: &Path, files: &[&str]) -> bool {
    let out = home.join("out");
    std::fs::create_dir_all(&out).unwrap();
    for f in files {
        std::fs::write(out.join(f), "old").unwrap();
    }
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o300)).unwrap();
    std::fs::read_dir(&out).is_err()
}

fn open_up(home: &Path) -> std::path::PathBuf {
    let out = home.join("out");
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o700)).unwrap();
    out
}

#[test]
fn the_dry_run_plans_the_names_the_run_writes_into_an_unlistable_folder() {
    let home = tempfile::tempdir().unwrap();
    library(home.path());
    if !drop_box(home.path(), &["Book.epub"]) {
        return; // root lists any folder; nothing to test
    }
    let dry = export(home.path(), &["--dry-run"]);
    assert_eq!(planned(&dry), ["Book (2).epub", "Book (3).epub"], "{dry}");
    let shown = export(home.path(), &[]);
    assert_eq!(planned(&shown), planned(&dry), "{shown}");
    let out = open_up(home.path());
    assert_eq!(books(&out), ["Book (2).epub", "Book (3).epub", "Book.epub"]);
    assert_eq!(std::fs::read(out.join("Book.epub")).unwrap(), b"old");
}

#[test]
fn a_plain_export_still_goes_into_an_empty_unlistable_folder() {
    let home = tempfile::tempdir().unwrap();
    library(home.path());
    if !drop_box(home.path(), &[]) {
        return;
    }
    let dry = export(home.path(), &["--dry-run"]);
    assert_eq!(planned(&dry), ["Book.epub", "Book (2).epub"], "{dry}");
    let shown = export(home.path(), &[]);
    assert_eq!(planned(&shown), planned(&dry), "{shown}");
    assert_eq!(books(&open_up(home.path())), ["Book (2).epub", "Book.epub"]);
}
