//! #209: `--detectors` that can't run warns and cuts without them (exit 0), while
//! `--strict-detectors` fails with the runtime's error before anything is written. With the `ml`
//! feature the models folder holds a corrupted runtime, and no download can replace it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;

const ID: &str = "series-1";
const WARNING: &str = "detectors off, cutting without them: ";
const CORE_WARNING: &str = "cutting without detectors";

/// The CLI with an empty certificate store, so the runtime can't be downloaded again.
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

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
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
    library::upsert_chapters(
        &lib,
        "mangadex",
        &manga.id,
        "en",
        std::slice::from_ref(&chapter),
    )
    .unwrap();
    let dir = home.path().join("chapters/1");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..2 {
        let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
        page.save(dir.join(format!("{i:03}.png"))).unwrap();
    }
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
    (home, dir)
}

/// The reason the default run prints after its warning; it exits 0 and writes the book.
fn fallback_reason(home: &Path, args: &[&str], written: &Path) -> String {
    let out = mangas(home, args);
    assert!(out.status.success(), "{out:?}");
    assert!(written.exists(), "{out:?}");
    let shown = stderr(&out);
    assert_eq!(shown.matches(WARNING).count(), 1, "{shown}");
    assert!(!shown.contains(CORE_WARNING), "{shown}");
    let (_, rest) = shown.split_once(WARNING).expect(&shown);
    let reason = rest.lines().next().unwrap_or_default().to_owned();
    assert!(!reason.is_empty(), "{shown}");
    reason
}

fn assert_strict_failure(out: &Output, reason: &str) {
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let shown = stderr(out);
    assert!(shown.contains("--strict-detectors"), "{shown}");
    assert!(shown.contains(reason), "{shown}");
    assert!(!shown.contains(WARNING), "{shown}");
    assert!(!shown.contains(CORE_WARNING), "{shown}");
}

#[test]
fn kindle_strict_fails_with_the_runtime_error_before_writing() {
    let (home, chapter) = home();
    let chapter = chapter.to_str().unwrap();
    let book = home.path().join("lenient.epub");
    let args = [
        "kindle",
        chapter,
        "--detectors",
        "-o",
        book.to_str().unwrap(),
    ];
    let reason = fallback_reason(home.path(), &args, &book);
    if cfg!(not(feature = "ml")) {
        assert!(reason.contains("`ml` feature"), "{reason}");
    }

    let strict = home.path().join("strict.epub");
    let out = mangas(
        home.path(),
        &["kindle", chapter, "--detectors", "--strict-detectors"]
            .into_iter()
            .chain(["-o", strict.to_str().unwrap()])
            .collect::<Vec<_>>(),
    );
    assert_strict_failure(&out, &reason);
    assert!(!strict.exists(), "{out:?}");
}

#[test]
fn export_strict_fails_with_the_runtime_error_before_writing() {
    let (home, _) = home();
    let lenient = home.path().join("lenient");
    let args = ["export", ID, "-o", lenient.to_str().unwrap(), "--detectors"];
    let reason = fallback_reason(home.path(), &args, &lenient);

    let strict = home.path().join("strict");
    let s = strict.to_str().unwrap();
    let args = ["export", ID, "-o", s, "--detectors", "--strict-detectors"];
    let out = mangas(home.path(), &args);
    assert_strict_failure(&out, &reason);
    let written = std::fs::read_dir(&strict).map_or(0, |d| d.count());
    assert_eq!(written, 0, "{out:?}");
}

#[test]
fn strict_needs_detectors_and_is_documented() {
    let (home, chapter) = home();
    let out = mangas(
        home.path(),
        &["kindle", chapter.to_str().unwrap(), "--strict-detectors"],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(stderr(&out).contains("--detectors"), "{out:?}");
    for cmd in ["kindle", "export"] {
        let help = mangas(home.path(), &[cmd, "--help"]);
        let help = String::from_utf8_lossy(&help.stdout).into_owned();
        assert!(help.contains("--strict-detectors"), "{cmd}: {help}");
        assert!(help.contains("exit 1"), "{cmd}: {help}");
    }
}
