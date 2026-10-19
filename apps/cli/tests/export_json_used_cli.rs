//! #218: `--json` reports `{"state": "used", "reason": null}` when the detectors ran. Downloads
//! the real runtime and models (~180 MB), so it is ignored by default:
//! `cargo test -p mangas-cli --features ml --test export_json_used_cli -- --ignored`.
#![cfg(feature = "ml")]

use std::process::Command;

use serde_json::{json, Value};

#[test]
#[ignore]
fn used_detectors_report_used() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("Series/1");
    std::fs::create_dir_all(&chapter).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(chapter.join("000.png")).unwrap();
    let book = home.path().join("book.epub");
    let out = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args([
            "kindle",
            chapter.to_str().unwrap(),
            "--detectors",
            "--json",
            "-o",
        ])
        .arg(&book)
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("\ndetectors: "), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["detectors"],
        json!({"state": "used", "reason": null})
    );
}
