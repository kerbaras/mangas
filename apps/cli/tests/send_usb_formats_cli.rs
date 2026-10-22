//! `mangas send --via usb` asks the format table before copying: PDF and AZW3 copy, EPUB only
//! with `--force`, CBZ never; `--json` reports the decision under `usb`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn send(home: &Path, book: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(["send", "--via", "usb", "--kindle"])
        .arg(home.join("Kindle"))
        .args(extra)
        .arg(book)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn setup(ext: &str) -> (tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("Kindle/documents")).unwrap();
    std::fs::create_dir_all(home.path().join("Kindle/system")).unwrap();
    let book = home.path().join(format!("Vol 1.{ext}"));
    std::fs::write(&book, b"book").unwrap();
    (home, book)
}

fn on_kindle(home: &Path, ext: &str) -> bool {
    home.join(format!("Kindle/documents/Vol 1.{ext}")).is_file()
}

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn pdf_and_azw3_copy_as_before_and_report_supported() {
    for ext in ["pdf", "azw3"] {
        let (home, book) = setup(ext);
        let out = send(home.path(), &book, &["--json"]);
        assert!(out.status.success(), "{out:?}");
        assert!(on_kindle(home.path(), ext));
        let r = json(&out);
        assert_eq!(r["sent"], true, "{r}");
        assert_eq!(r["usb"]["decision"], "supported", "{r}");
        assert_eq!(r["usb"]["format"], ext.to_uppercase(), "{r}");
        assert_eq!(r["usb"]["device"], "Kindle", "{r}");
        assert!(r["usb"]["reason"].is_null(), "{r}");
    }
}

#[test]
fn a_cbz_is_refused_even_with_force() {
    let (home, book) = setup("cbz");
    for extra in [&[][..], &["--force"][..]] {
        let out = send(home.path(), &book, extra);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let err = stderr(&out);
        assert!(err.contains("Kindle doesn't open CBZ files"), "{err}");
        assert!(err.contains("export the book as PDF or AZW3"), "{err}");
        assert!(!err.contains("--force"), "{err}");
    }
    assert!(!on_kindle(home.path(), "cbz"));
    let r = json(&send(home.path(), &book, &["--json"]));
    assert_eq!(r["sent"], false, "{r}");
    assert_eq!(r["usb"]["decision"], "refused", "{r}");
    assert!(r["usb"]["reason"].as_str().unwrap().contains("CBZ"), "{r}");
    assert!(r["error"].is_string(), "{r}");
}

#[test]
fn an_epub_warns_and_copies_only_with_force() {
    let (home, book) = setup("epub");
    let out = send(home.path(), &book, &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = stderr(&out);
    assert!(
        err.contains("Kindle may not open an EPUB copied over USB"),
        "{err}"
    );
    assert!(err.contains("Email it instead"), "{err}");
    assert!(
        err.contains("copy it anyway with: mangas send --via usb --force"),
        "{err}"
    );
    assert!(
        err.contains("or email it: mangas send --via email"),
        "{err}"
    );
    assert!(!on_kindle(home.path(), "epub"));
    let r = json(&send(home.path(), &book, &["--json"]));
    assert_eq!(
        (r["sent"].clone(), r["usb"]["decision"].clone()),
        (false.into(), "warn".into())
    );

    let out = send(home.path(), &book, &["--force", "--json"]);
    assert!(out.status.success(), "{out:?}");
    assert!(on_kindle(home.path(), "epub"));
    assert!(
        stderr(&out).contains("warning: Kindle may not open an EPUB"),
        "{out:?}"
    );
    let r = json(&out);
    assert_eq!(r["sent"], true, "{r}");
    assert_eq!(r["usb"]["decision"], "warn", "{r}");
    assert!(r["usb"]["reason"].is_string(), "{r}");
}

#[test]
fn email_reports_no_usb_decision() {
    let (home, book) = setup("pdf");
    let out = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(["send", "--json"])
        .arg(&book)
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .output()
        .unwrap();
    assert!(!out.status.success(), "no email settings: {out:?}");
    let r = json(&out);
    assert_eq!(r["via"], "email", "{r}");
    assert!(r["usb"].is_null(), "{r}");
}
