//! With a Kindle on the bus over MTP only (no mounted folder), the USB format table still
//! decides first: a CBZ is refused as a CBZ (never pointed at Email, which doesn't take it),
//! an export writes nothing for it, and `send --json` reports the decision.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const DEVICE: &str = "Kindle Paperwhite";

/// The bus is faked, but mounts are real: a Kindle mounted on this machine would be picked.
fn mounted_kindle() -> bool {
    let mounted = !mangas_core::send::usb::find_kindles().is_empty();
    if mounted {
        eprintln!("skipped: a Kindle is mounted here");
    }
    mounted
}

fn cli(home: &Path, args: &[&str], tail: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .args(tail)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_FAKE_USB_KINDLE", DEVICE)
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn book(home: &Path, ext: &str) -> PathBuf {
    let book = home.join(format!("Vol 1.{ext}"));
    std::fs::write(&book, b"book").unwrap();
    book
}

fn send(home: &Path, ext: &str, extra: &[&str]) -> Output {
    let book = book(home, ext);
    cli(home, &[&["send", "--via", "usb"], extra].concat(), &[&book])
}

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_cbz_is_refused_by_format_with_no_email_suggestion() {
    if mounted_kindle() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let out = send(home.path(), "cbz", &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = stderr(&out);
    assert!(err.contains(&format!("{DEVICE} doesn't open CBZ")), "{err}");
    assert!(err.contains("PDF or AZW3"), "{err}");
    assert!(!err.contains("email") && !err.contains("MTP"), "{err}");
    let r = json(&send(home.path(), "cbz", &["--json"]));
    assert_eq!(r["sent"], false, "{r}");
    assert_eq!(r["usb"]["decision"], "refused", "{r}");
    assert_eq!(r["usb"]["device"], DEVICE, "{r}");
    assert_eq!(r["to"], DEVICE, "{r}");
}

#[test]
fn supported_formats_meet_mtp_and_only_a_pdf_is_pointed_at_email() {
    if mounted_kindle() {
        return;
    }
    for (ext, email) in [("pdf", true), ("azw3", false)] {
        let home = tempfile::tempdir().unwrap();
        let out = send(home.path(), ext, &[]);
        assert!(!out.status.success(), "{out:?}");
        let err = stderr(&out);
        assert!(err.contains("connects over MTP"), "{err}");
        assert_eq!(
            err.contains("mangas send --via email"),
            email,
            "{ext}: {err}"
        );
        assert_eq!(
            !err.contains("export the book as PDF or EPUB"),
            email,
            "{err}"
        );
        let r = json(&send(home.path(), ext, &["--json"]));
        assert_eq!(r["sent"], false, "{r}");
        assert_eq!(r["usb"]["decision"], "supported", "{r}");
        assert_eq!(r["usb"]["format"], ext.to_uppercase(), "{r}");
        assert_eq!(r["usb"]["device"], DEVICE, "{r}");
    }
}

#[test]
fn an_epub_is_refused_by_format_even_forced_and_pointed_at_email() {
    if mounted_kindle() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    for extra in [&[][..], &["--force"][..]] {
        let out = send(home.path(), "epub", extra);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let err = stderr(&out);
        assert!(
            err.contains(&format!("{DEVICE} doesn't open an EPUB")),
            "{err}"
        );
        assert!(err.contains("mangas send --via email"), "{err}");
        assert!(!err.contains("MTP"), "{err}");
    }
    let r = json(&send(home.path(), "epub", &["--json"]));
    assert_eq!(r["usb"]["decision"], "refused", "{r}");
}

/// `mangas kindle <chapter> --format <ext> [--json] --send usb -o out/`, no `--kindle`.
fn kindle(home: &Path, ext: &str, json: bool) -> Output {
    let chapter = home.join("ch1");
    std::fs::create_dir_all(&chapter).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(chapter.join("001.png")).unwrap();
    let args = [
        "kindle", "--format", ext, "--title", "Vol 1", "--send", "usb",
    ];
    let json = if json { &["--json"][..] } else { &[] };
    let out = home.join("out/");
    cli(
        home,
        &[&args[..], json].concat(),
        &[&chapter, Path::new("-o"), &out],
    )
}

#[test]
fn a_cbz_export_writes_nothing_and_reports_it() {
    if mounted_kindle() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let out = kindle(home.path(), "cbz", false);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(!stderr(&out).contains("email"), "{out:?}");
    assert!(!home.path().join("out").exists(), "nothing written");
    let r = json(&kindle(home.path(), "cbz", true));
    assert_eq!(r["files"], serde_json::json!([]), "{r}");
    assert_eq!(r["not_written"][0]["error"], "usb_refused", "{r}");
    assert!(!home.path().join("out").exists(), "nothing written");
}

#[test]
fn a_pdf_export_is_written_then_pointed_at_email() {
    if mounted_kindle() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let out = kindle(home.path(), "pdf", false);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let err = stderr(&out);
    assert!(err.contains("was written but not sent"), "{err}");
    assert!(err.contains("mangas send --via email"), "{err}");
    assert!(home.path().join("out/Vol 1.pdf").is_file(), "{out:?}");
}
