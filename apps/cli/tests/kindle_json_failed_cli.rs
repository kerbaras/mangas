//! #314: `mangas kindle --json` whose export fails still prints its one JSON object, with the
//! book in `failed` and the error as its reason, like `mangas export --json`.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

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

#[test]
fn a_failed_kindle_export_prints_the_partial_report() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("Series/Chapter 1");
    std::fs::create_dir_all(&chapter).unwrap();
    for i in 0..2 {
        let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
        page.save(chapter.join(format!("{i:03}.png"))).unwrap();
    }
    let locked = home.path().join("read-only");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(locked.join("probe"), b"").is_ok() {
        eprintln!("skipped: running as a user who can write anywhere");
        return;
    }
    let book = locked.join("Series.epub");
    let args = [
        "kindle",
        chapter.to_str().unwrap(),
        "-o",
        book.to_str().unwrap(),
        "--json",
    ];
    let out = mangas(home.path(), &args);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!out.status.success(), "{out:?}");
    let report: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    assert_eq!(report["files"], serde_json::json!([]), "{report}");
    let failed = report["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 1, "{report}");
    assert_eq!(failed[0]["path"], book.to_str().unwrap());
    let reason = failed[0]["reason"].as_str().unwrap();
    assert!(reason.contains("ermission denied"), "{reason}");
    assert_eq!(report["not_written"], serde_json::json!([]));
    assert!(report["detectors"]["state"].is_string(), "{report}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(reason), "{stderr}");
}
