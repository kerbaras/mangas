//! #330: `--eject` on `send`, `export --send usb` and its `eject` JSON object. A fake Kindle (a
//! plain folder) is never a drive of its own, so the eject is refused, says so, and leaves the
//! book on it.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn send_eject_on_a_folder_is_refused_and_keeps_the_book() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let book = home.path().join("Vol 1.azw3");
    std::fs::write(&book, b"azw3 bytes").unwrap();
    let args = [
        "send",
        "--via",
        "usb",
        "--eject",
        "--kindle",
        kindle.to_str().unwrap(),
        book.to_str().unwrap(),
    ];
    let out = mangas(home.path(), &args);
    assert!(!out.status.success(), "{out:?}");
    assert!(text(&out.stdout).contains("copied to"), "{out:?}");
    let err = text(&out.stderr);
    assert!(err.contains("can't eject"), "{err}");
    assert!(err.contains("isn't a drive of its own"), "{err}");
    assert!(
        err.contains("The books copied to it are still there"),
        "{err}"
    );
    assert_eq!(
        std::fs::read(kindle.join("documents/Vol 1.azw3")).unwrap(),
        b"azw3 bytes"
    );
}

#[test]
fn send_eject_without_a_file_ejects_the_given_kindle() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let out = mangas(
        home.path(),
        &[
            "send",
            "--via",
            "usb",
            "--eject",
            "--kindle",
            kindle.to_str().unwrap(),
        ],
    );
    assert!(!out.status.success(), "{out:?}");
    assert!(text(&out.stderr).contains("can't eject"), "{out:?}");
}

#[test]
fn send_eject_without_a_file_by_email_ejects_nothing() {
    let home = tempfile::tempdir().unwrap();
    let out = mangas(home.path(), &["send", "--eject"]);
    assert!(!out.status.success(), "{out:?}");
    assert!(text(&out.stderr).contains("add --via usb"), "{out:?}");
}

#[test]
fn eject_is_refused_with_test_or_devices() {
    let home = tempfile::tempdir().unwrap();
    for other in ["--test", "--devices"] {
        let out = mangas(home.path(), &["send", other, "--eject"]);
        assert_eq!(out.status.code(), Some(2), "{other}: {out:?}");
    }
}

/// The `eject` object `mangas kindle --json` prints for a fake chapter folder.
fn kindle_json(home: &Path, extra: &[&str]) -> (Output, Value) {
    let chapter = home.join("chapter");
    std::fs::create_dir_all(&chapter).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(chapter.join("001.png")).unwrap();
    let out_dir = home.join("out");
    let mut args = vec!["kindle", chapter.to_str().unwrap(), "--json", "-o"];
    args.push(out_dir.to_str().unwrap());
    args.extend_from_slice(extra);
    let out = mangas(home, &args);
    let json = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    (out, json)
}

#[test]
fn kindle_json_reports_the_eject() {
    let home = tempfile::tempdir().unwrap();
    let (_, json) = kindle_json(home.path(), &[]);
    let none = serde_json::json!({ "attempted": false, "ok": false, "error": null });
    assert_eq!(json["eject"], none, "{json}");

    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let mount = kindle.to_str().unwrap().to_string();
    let (out, json) = kindle_json(
        home.path(),
        &["--send", "usb", "--kindle", &mount, "--eject", "--force"],
    );
    assert!(!out.status.success(), "{out:?}");
    assert_eq!(json["eject"]["attempted"], true, "{json}");
    assert_eq!(json["eject"]["ok"], false, "{json}");
    let error = json["eject"]["error"].as_str().unwrap_or_default();
    assert!(error.contains("isn't a drive of its own"), "{json}");
    let copied = std::fs::read_dir(kindle.join("documents")).unwrap().count();
    assert_eq!(copied, 1, "the book stays on the Kindle");
}

#[test]
fn send_json_reports_the_eject() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let book = home.path().join("Vol 1.pdf");
    std::fs::write(&book, b"%PDF bytes").unwrap();
    let k = kindle.to_str().unwrap();
    let args = ["send", "--via", "usb", "--kindle", k, "--json", "--eject"];
    let out = mangas(
        home.path(),
        &[&args[..], &[book.to_str().unwrap()]].concat(),
    );
    assert!(!out.status.success(), "{out:?}");
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["sent"], true, "{json}");
    assert_eq!(json["eject"]["attempted"], true, "{json}");
    assert_eq!(json["eject"]["ok"], false, "{json}");
    let error = json["eject"]["error"].as_str().unwrap_or_default();
    assert!(error.contains("isn't a drive of its own"), "{json}");
    assert!(kindle.join("documents/Vol 1.pdf").is_file());
}

#[test]
fn refused_send_still_ejects_and_says_why_the_kindle_stays() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let book = home.path().join("Vol 1.epub");
    std::fs::write(&book, b"epub bytes").unwrap();
    let k = kindle.to_str().unwrap();
    let args = [
        "send",
        "--via",
        "usb",
        "--eject",
        "--kindle",
        k,
        book.to_str().unwrap(),
    ];
    let out = mangas(home.path(), &args);
    assert!(!out.status.success(), "{out:?}");
    let err = text(&out.stderr);
    assert!(err.contains("doesn't open an EPUB"), "{err}");
    assert!(err.contains("can't eject"), "{err}");
    assert!(err.contains("isn't a drive of its own"), "{err}");
    let copied = std::fs::read_dir(kindle.join("documents")).unwrap().count();
    assert_eq!(copied, 0, "a refused send copies nothing");
}
