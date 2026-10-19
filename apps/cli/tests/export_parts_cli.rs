//! #134: `--split-mb` writes numbered parts, `--json` reports each with its size, and `--send
//! email` delivers every part, each under its own subject.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;
use serde_json::Value;

mod support;
use support::{noisy_chapter, smtp_sessions};

const ID: &str = "series-1";

fn mangas(home: &Path, args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(stdin.as_bytes()).unwrap();
    drop(input);
    child.wait_with_output().unwrap()
}

/// Follows a manga with chapters "1" and "2" downloaded, three noisy pages each.
fn library_with_noisy_chapters(home: &Path) {
    let db = home.join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let manga = Manga {
        id: MangaId(ID.into()),
        source: "mangadex".into(),
        title: "Series".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = |n: &str| Chapter {
        id: ChapterId(format!("c{n}")),
        manga_id: MangaId(ID.into()),
        number: n.into(),
        language: "en".into(),
        ..Default::default()
    };
    let chapters = [chapter("1"), chapter("2")];
    library::upsert_chapters(&lib, "mangadex", &manga.id, "en", &chapters).unwrap();
    for c in &chapters {
        let dir = home.join("chapters").join(&c.number);
        noisy_chapter(&dir, 3);
        library::mark_downloaded(&lib, "mangadex", &c.id, &dir).unwrap();
    }
}

fn report(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

/// Every `files` entry is a numbered part of at most `max` bytes, as big as the file on disk.
fn assert_parts(r: &Value, stem: &str, max: u64) -> Vec<PathBuf> {
    let files = r["files"].as_array().unwrap();
    assert!(files.len() >= 2, "{r}");
    let total = files.len() as u64;
    files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let path = PathBuf::from(f["path"].as_str().unwrap());
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            assert_eq!(name, format!("{stem} (part {}).epub", i + 1));
            assert_eq!(
                f["part"],
                serde_json::json!({ "index": i + 1, "total": total })
            );
            let bytes = std::fs::metadata(&path).unwrap().len();
            assert_eq!(f["bytes"], bytes, "{name}");
            assert!(bytes <= max, "{name}: {bytes} > {max}");
            path
        })
        .collect()
}

#[test]
fn kindle_json_reports_every_part_under_the_split_size() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("chapters/1");
    noisy_chapter(&chapter, 6);
    let book = home.path().join("book.epub");
    let args = [
        "kindle",
        chapter.to_str().unwrap(),
        "--json",
        "--split-mb",
        "1",
        "-o",
    ];
    let mut args: Vec<&str> = args.to_vec();
    args.push(book.to_str().unwrap());
    let out = mangas(home.path(), &args, "");
    assert!(out.status.success(), "{out:?}");
    let r = report(&out);
    let parts = assert_parts(&r, "book", 1_000_000);
    let pages: u64 = r["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["pages"].as_u64().unwrap())
        .sum();
    assert_eq!(pages, 6, "{r}");
    assert!(!book.exists(), "no whole book besides the parts");
    assert!(parts.iter().all(|p| p.exists()));
}

#[test]
fn a_lower_quality_writes_a_smaller_book() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("chapters/1");
    noisy_chapter(&chapter, 2);
    let size = |quality: &str, name: &str| {
        let book = home.path().join(name);
        let args = [
            "kindle",
            chapter.to_str().unwrap(),
            "--quality",
            quality,
            "-o",
        ];
        let mut args: Vec<&str> = args.to_vec();
        args.push(book.to_str().unwrap());
        let out = mangas(home.path(), &args, "");
        assert!(out.status.success(), "{out:?}");
        std::fs::metadata(&book).unwrap().len()
    };
    assert!(size("50", "low.epub") < size("90", "high.epub"));
    let out = mangas(home.path(), &["kindle", "x", "--quality", "20"], "");
    assert!(!out.status.success());
}

#[test]
fn export_emails_every_part_under_its_own_subject() {
    let home = tempfile::tempdir().unwrap();
    library_with_noisy_chapters(home.path());
    let out_dir = home.path().join("out");
    // One session per part: two ~0.9 MB pages fit in 2 MB, so six pages make three parts.
    let (port, smtp) = smtp_sessions(3);
    let port = port.to_string();
    let email = [
        "settings",
        "email",
        "--to",
        "reader@kindle.com",
        "--host",
        "127.0.0.1",
        "--port",
        &port,
        "--security",
        "none",
        "--username",
        "me@example.test",
        "--password-stdin",
    ];
    assert!(mangas(home.path(), &email, "pw\n").status.success());
    let args = [
        "export",
        ID,
        "--json",
        "--split-mb",
        "2",
        "--send",
        "email",
        "-o",
    ];
    let mut args: Vec<&str> = args.to_vec();
    args.push(out_dir.to_str().unwrap());
    let out = mangas(home.path(), &args, "");
    assert!(out.status.success(), "{out:?}");
    let r = report(&out);
    assert_eq!(r["failed"], serde_json::json!([]), "{r}");
    let parts = assert_parts(&r, "Series ch 1-2", 2_000_000);
    assert_eq!(parts.len(), 3, "{r}");
    let log = smtp.join().unwrap();
    assert_eq!(log.lines().filter(|l| *l == "DATA").count(), 3, "{log}");
    for n in 1..=3 {
        assert!(
            log.contains(&format!("Subject: Series ch 1-2 (part {n} of 3)")),
            "{log}"
        );
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.matches("emailed").count(), 3, "{stderr}");
}
