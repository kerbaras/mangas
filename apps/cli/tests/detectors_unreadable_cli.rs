//! `mangas kindle --detectors` with the stored settings row broken: a warning, then the detector
//! step runs on the defaults (default data folder, system proxy) instead of failing on the row.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::library;

fn kindle_with_detectors(home: &Path, chapter: &Path) -> Output {
    let out = home.join("out.epub");
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(["kindle", chapter.to_str().unwrap(), "--detectors", "-o"])
        .arg(out)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        // A dead system proxy: an `ml` build's detector download fails fast through it.
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

#[test]
fn a_broken_settings_row_falls_back_to_the_defaults() {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    drop(library::open(&db).unwrap());
    let broken = "INSERT OR REPLACE INTO setting (key, value) VALUES ('app', '{broken')";
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(broken, [])
        .unwrap();
    let chapter = home.path().join("ch1");
    std::fs::create_dir_all(&chapter).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(chapter.join("000.png")).unwrap();

    let out = kindle_with_detectors(home.path(), &chapter);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let warning = "warning: could not read the settings, using the default data folder and the \
                   system proxy: settings: key must be a string";
    assert!(stderr.contains(warning), "{stderr}");
    let last = stderr.lines().last().unwrap_or_default();
    assert!(!last.contains("settings:"), "{stderr}");
}
