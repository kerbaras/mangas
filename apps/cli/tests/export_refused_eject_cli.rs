//! #330 with #420: an export whose delivery is refused copies nothing, and `--eject` still
//! ejects the Kindle (the user is done with it), reported in `--json` `eject`. Nothing mounted
//! (Email, a Kindle on MTP only) means nothing was attempted. A fake Kindle (a plain folder) is
//! never a drive of its own, so its eject is refused; `MANGAS_TEST_KINDLE` (a mounted Kindle
//! volume) runs the ejected case on a real drive.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;
use serde_json::{json, Value};

fn cli(home: &Path, args: &[&str], mtp: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mangas-cli"));
    if let Some(device) = mtp {
        cmd.env("MANGAS_FAKE_USB_KINDLE", device);
    }
    cmd.args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

/// stdout as the one JSON object it must be, the exit failed, nothing written.
fn refused(out: &Output, class: &str) -> Value {
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let r: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    assert_eq!(r["files"], json!([]), "{r}");
    assert_eq!(r["not_written"][0]["error"], class, "{r}");
    r
}

fn not_attempted() -> Value {
    json!({ "attempted": false, "ok": false, "error": null })
}

fn page(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("001.png")).unwrap();
}

/// A followed manga with one downloaded chapter.
fn library_with_a_chapter(home: &Path) {
    let db = home.join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let manga = Manga {
        id: MangaId("series-1".into()),
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
    let one = std::slice::from_ref(&chapter);
    library::upsert_chapters(&lib, "mangadex", &manga.id, "en", one).unwrap();
    let dir = home.join("chapters/1");
    page(&dir);
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
}

#[test]
fn refused_epub_over_usb_still_ejects_the_kindle() {
    let home = tempfile::tempdir().unwrap();
    library_with_a_chapter(home.path());
    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let (k, out) = (kindle.to_str().unwrap(), home.path().join("out"));
    let o = out.to_str().unwrap();
    let args = [
        "export", "series-1", "--format", "epub", "--send", "usb", "--kindle", k, "--eject",
        "--json", "-o", o,
    ];
    let ran = cli(home.path(), &args, None);
    let r = refused(&ran, "usb_refused");
    assert_eq!(r["eject"]["attempted"], true, "{r}");
    assert_eq!(r["eject"]["ok"], false, "{r}");
    let error = r["eject"]["error"].as_str().unwrap_or_default();
    assert!(error.contains("isn't a drive of its own"), "{r}");
    let err = String::from_utf8_lossy(&ran.stderr);
    assert!(
        err.contains("--format azw3") && err.contains("can't eject"),
        "{err}"
    );
    assert!(!out.exists(), "the refused export wrote {o}");
}

#[test]
fn refused_email_with_eject_attempts_nothing() {
    let home = tempfile::tempdir().unwrap();
    library_with_a_chapter(home.path());
    let o = home.path().join("out");
    let args = [
        "export",
        "series-1",
        "--format",
        "azw3",
        "--send",
        "email",
        "--eject",
        "--json",
        "-o",
        o.to_str().unwrap(),
    ];
    let ran = cli(home.path(), &args, None);
    let r = refused(&ran, "email_refused");
    assert_eq!(r["eject"], not_attempted(), "{r}");
    let err = String::from_utf8_lossy(&ran.stderr);
    assert!(err.contains("--eject"), "{err}");
}

#[test]
fn kindle_on_mtp_only_with_eject_attempts_nothing() {
    if !mangas_core::send::usb::find_kindles().is_empty() {
        return eprintln!("skipped: a Kindle is mounted here");
    }
    let home = tempfile::tempdir().unwrap();
    library_with_a_chapter(home.path());
    let o = home.path().join("out");
    let args = [
        "export",
        "series-1",
        "--send",
        "usb",
        "--eject",
        "--json",
        "-o",
        o.to_str().unwrap(),
    ];
    let ran = cli(home.path(), &args, Some("Kindle Paperwhite"));
    let r = refused(&ran, "kindle_over_mtp");
    assert_eq!(r["eject"], not_attempted(), "{r}");
    let err = String::from_utf8_lossy(&ran.stderr);
    assert!(err.contains("no mounted Kindle to eject"), "{err}");
}

/// `kindle --format epub --send usb --eject` on a real drive: refused, nothing copied, ejected.
/// Needs a mounted Kindle volume: `MANGAS_TEST_KINDLE=/media/$USER/KINDLE cargo test -- --ignored`.
#[test]
#[ignore = "needs MANGAS_TEST_KINDLE, a mounted Kindle volume"]
fn refused_kindle_export_ejects_a_real_drive() {
    let Some(mount) = std::env::var_os("MANGAS_TEST_KINDLE") else {
        return;
    };
    let home = tempfile::tempdir().unwrap();
    let (chapter, out) = (home.path().join("ch1"), home.path().join("out/"));
    page(&chapter);
    let k = mount.to_str().unwrap();
    let args = [
        "kindle",
        chapter.to_str().unwrap(),
        "--format",
        "epub",
        "--send",
        "usb",
        "--kindle",
        k,
        "--eject",
        "--json",
        "-o",
        out.to_str().unwrap(),
    ];
    let r = refused(&cli(home.path(), &args, None), "usb_refused");
    let ok = json!({ "attempted": true, "ok": true, "error": null });
    assert_eq!(r["eject"], ok, "{r}");
    assert!(
        !Path::new(k).join("documents").exists(),
        "{k} is still mounted"
    );
}
