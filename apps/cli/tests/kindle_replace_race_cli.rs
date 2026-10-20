//! #134: `kindle --replace --split-mb` runs replacing the same book at once each stage their own
//! parts, so every run finishes and the folder holds only whole parts.

use std::path::Path;
use std::process::{Child, Command, Stdio};

mod support;
use support::noisy_chapter;

fn kindle(home: &Path, chapter: &Path, book: &Path, replace: bool) -> Child {
    let mut args = vec![
        "kindle",
        chapter.to_str().unwrap(),
        "--split-mb",
        "1",
        "--json",
    ];
    if replace {
        args.push("--replace");
    }
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .arg("-o")
        .arg(book)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn finish(child: Child, what: &str) {
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{what}: {stderr}");
}

/// A whole zip ends with its end-of-central-directory record.
fn is_whole_zip(bytes: &[u8]) -> bool {
    let tail = &bytes[bytes.len().saturating_sub(64)..];
    bytes.starts_with(b"PK\x03\x04") && tail.windows(4).any(|w| w == b"PK\x05\x06")
}

#[test]
fn concurrent_replacements_of_one_book_never_share_staging_files() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("chapters/1");
    noisy_chapter(&chapter, 8);
    let out = home.path().join("out");
    let book = out.join("book.epub");
    finish(kindle(home.path(), &chapter, &book, false), "first export");
    let mut names: Vec<String> = Vec::new();
    for round in 0..3 {
        let runs: Vec<Child> = (0..4)
            .map(|_| kindle(home.path(), &chapter, &book, true))
            .collect();
        for (i, run) in runs.into_iter().enumerate() {
            finish(run, &format!("round {round}, run {i}"));
        }
        names = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| !n.ends_with(".part")), "{names:?}");
    }
    names.sort();
    assert!(names.len() >= 2, "{names:?}");
    for (i, name) in names.iter().enumerate() {
        assert_eq!(name, &format!("book (part {}).epub", i + 1));
        assert!(
            is_whole_zip(&std::fs::read(out.join(name)).unwrap()),
            "{name}"
        );
    }
}
