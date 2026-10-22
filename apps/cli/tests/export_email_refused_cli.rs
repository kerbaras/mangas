//! `--send email` with a format Send to Kindle by email doesn't take (AZW3, CBZ): `kindle` and
//! `export` stop before writing anything, name the format and the route, and point at EPUB.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;
use serde_json::Value;

fn cli(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn page(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("001.png")).unwrap();
}

fn kindle(home: &Path, ext: &str, json: bool) -> Output {
    let (chapter, out) = (home.join("ch1"), home.join("out/"));
    page(&chapter);
    let (chapter, out) = (chapter.to_str().unwrap(), out.to_str().unwrap());
    let args = [
        "kindle", chapter, "--format", ext, "--send", "email", "-o", out,
    ];
    cli(
        home,
        &[&args[..], if json { &["--json"] } else { &[] }].concat(),
    )
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
    library::upsert_chapters(
        &lib,
        "mangadex",
        &manga.id,
        "en",
        std::slice::from_ref(&chapter),
    )
    .unwrap();
    let dir = home.join("chapters/1");
    page(&dir);
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
}

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn kindle_writes_nothing_for_an_azw3_or_cbz_by_email() {
    for ext in ["azw3", "cbz"] {
        let home = tempfile::tempdir().unwrap();
        let out = kindle(home.path(), ext, false);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let err = stderr(&out);
        let what = format!(
            "by email takes EPUB or PDF, not a{} {}",
            "n".repeat((ext == "azw3") as usize),
            ext.to_uppercase()
        );
        assert!(err.contains(&what), "{err}");
        assert!(err.contains("export with --format epub"), "{err}");
        assert!(!home.path().join("out").exists(), "nothing written");
        let r = json(&kindle(home.path(), ext, true));
        assert_eq!(r["files"], serde_json::json!([]), "{r}");
        assert_eq!(r["not_written"][0]["error"], "email_refused", "{r}");
        assert!(
            r["not_written"][0]["reason"]
                .as_str()
                .unwrap()
                .contains(&what),
            "{r}"
        );
        assert!(!home.path().join("out").exists(), "nothing written");
    }
}

#[test]
fn export_writes_nothing_for_an_azw3_by_email() {
    let home = tempfile::tempdir().unwrap();
    library_with_a_chapter(home.path());
    let out_dir = home.path().join("out");
    let out = out_dir.to_str().unwrap();
    let args = [
        "export", "series-1", "--format", "azw3", "--send", "email", "-o", out,
    ];
    let text = cli(home.path(), &args);
    assert_eq!(text.status.code(), Some(1), "{text:?}");
    assert!(stderr(&text).contains("not an AZW3"), "{text:?}");
    let r = json(&cli(home.path(), &[&args[..], &["--json"]].concat()));
    assert_eq!(r["files"], serde_json::json!([]), "{r}");
    assert_eq!(r["not_written"][0]["error"], "email_refused", "{r}");
    assert!(
        r["not_written"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with(".azw3"),
        "{r}"
    );
    assert!(!out_dir.exists(), "nothing written");
}

#[test]
fn an_epub_by_email_is_not_stopped_by_the_format() {
    let home = tempfile::tempdir().unwrap();
    let out = kindle(home.path(), "epub", false);
    assert!(
        !stderr(&out).contains("by email takes EPUB or PDF"),
        "{out:?}"
    );
    assert!(home.path().join("out").exists(), "{out:?}");
}
