//! #218: `mangas kindle --json` and `mangas export --json` print one JSON object: the books
//! written, chapters left out, books not sent and the detectors' state ("off", "skipped" with the
//! runtime's error, "used"). With the `ml` feature the models folder holds a corrupted runtime and
//! the certificate store is empty, so the detectors are skipped without a download. "used" is in
//! `export_json_used_cli.rs`, sending and failed volumes in `export_json_send_cli.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;
use serde_json::{json, Value};

const ID: &str = "series-1";
const WARNING: &str = "detectors off, cutting without them: ";

fn mangas(home: &Path, args: &[&str]) -> Output {
    let no_certs = home.join("no-certs");
    std::fs::create_dir_all(&no_certs).unwrap();
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("SSL_CERT_FILE", no_certs.join("none.pem"))
        .env("SSL_CERT_DIR", &no_certs)
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

#[cfg(feature = "ml")]
fn corrupt_runtime(home: &Path) {
    use mangas_core::ml::{cache::model_path, runtime, BUBBLE_TEXT_DETECTOR, FACE_DETECTOR};
    let models = home.join("data/mangas/models");
    let lib = runtime::lib_path(&models, &runtime::current().unwrap());
    std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
    std::fs::write(&lib, b"not a library").unwrap();
    for m in [&BUBBLE_TEXT_DETECTOR, &FACE_DETECTOR] {
        let path = model_path(&models, m);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"not a model").unwrap();
    }
}

#[cfg(not(feature = "ml"))]
fn corrupt_runtime(_home: &Path) {}

/// A home with a broken runtime and a followed manga whose chapter "1" is downloaded.
fn home() -> (tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir().unwrap();
    corrupt_runtime(home.path());
    let db = home.path().join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let manga = Manga {
        id: MangaId(ID.into()),
        source: "mangadex".into(),
        title: "Series".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("c1".into()),
        manga_id: manga.id.clone(),
        number: "1".into(),
        language: "en".into(),
        ..Default::default()
    };
    let chapters = std::slice::from_ref(&chapter);
    library::upsert_chapters(&lib, "mangadex", &manga.id, "en", chapters).unwrap();
    let dir = home.path().join("chapters/1");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..2 {
        let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
        page.save(dir.join(format!("{i:03}.png"))).unwrap();
    }
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
    (home, dir)
}

/// The whole of stdout, parsed: nothing but the one object.
fn report(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The reason after the human warning on stderr.
fn warned_reason(out: &Output) -> String {
    let shown = stderr(out);
    let (_, rest) = shown.split_once(WARNING).expect(&shown);
    rest.lines().next().unwrap_or_default().to_owned()
}

#[test]
fn kindle_json_lists_the_book_the_empty_chapter_and_detectors_off() {
    let (home, chapter) = home();
    let empty = home.path().join("chapters/2");
    std::fs::create_dir_all(&empty).unwrap();
    let book = home.path().join("book.epub");
    let (c, e, b) = (
        chapter.to_str().unwrap(),
        empty.to_str().unwrap(),
        book.to_str().unwrap(),
    );
    let out = mangas(home.path(), &["kindle", c, e, "-o", b, "--json"]);
    assert!(out.status.success(), "{out:?}");
    let r = report(&out);
    assert_eq!(r["files"][0]["path"], b);
    assert_eq!(r["files"][0]["title"], "Series");
    assert_eq!(r["files"][0]["chapters"], 1);
    assert!(r["files"][0]["pages"].as_u64().unwrap() > 0, "{r}");
    assert_eq!(r["files"][0]["skipped_files"], 0);
    assert_eq!(r["files"][0]["jpeg_quality"], 90);
    assert_eq!(
        r["skipped"],
        json!([{"path": e, "reason": "no readable pages"}])
    );
    assert_eq!(r["failed"], json!([]));
    assert_eq!(r["detectors"], json!({"state": "off", "reason": null}));
    assert!(book.is_file());
}

#[test]
fn skipped_detectors_report_the_reason_the_warning_shows() {
    let (home, chapter) = home();
    let book = home.path().join("book.epub");
    let (c, b) = (chapter.to_str().unwrap(), book.to_str().unwrap());
    let out = mangas(
        home.path(),
        &["kindle", c, "--detectors", "-o", b, "--json"],
    );
    assert!(out.status.success(), "{out:?}");
    let reason = warned_reason(&out);
    if cfg!(not(feature = "ml")) {
        assert!(reason.contains("`ml` feature"), "{reason}");
    }
    let skipped = json!({"state": "skipped", "reason": reason});
    assert_eq!(report(&out)["detectors"], skipped);

    let dir = home.path().join("books");
    let args = [
        "export",
        ID,
        "-o",
        dir.to_str().unwrap(),
        "--detectors",
        "--json",
    ];
    let out = mangas(home.path(), &args);
    assert!(out.status.success(), "{out:?}");
    let r = report(&out);
    assert_eq!(r["detectors"], skipped);
    assert_eq!(
        r["files"][0]["path"],
        dir.join("Series ch 1.epub").to_str().unwrap()
    );
    assert_eq!(r["files"][0]["chapters"], 1);
}

#[test]
fn export_json_lists_a_book_that_was_not_sent_and_exits_2() {
    let (home, _) = home();
    let dir = home.path().join("books");
    let d = dir.to_str().unwrap();
    let args = ["export", ID, "-o", d, "--json", "--send", "email"];
    let out = mangas(home.path(), &args);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let r = report(&out);
    let book = dir.join("Series ch 1.epub");
    assert_eq!(r["files"][0]["path"], book.to_str().unwrap());
    assert_eq!(r["failed"][0]["path"], book.to_str().unwrap());
    let reason = r["failed"][0]["reason"].as_str().unwrap();
    assert!(reason.contains("was written but not sent"), "{reason}");
    assert_eq!(r["detectors"], json!({"state": "off", "reason": null}));
}

#[test]
fn json_conflicts_with_the_modes_that_write_nothing() {
    let (home, chapter) = home();
    let c = chapter.to_str().unwrap();
    for args in [
        &["export", ID, "--json", "--preset", "show"][..],
        &["kindle", c, "--json", "--list-profiles"],
    ] {
        let out = mangas(home.path(), args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
    }
}
