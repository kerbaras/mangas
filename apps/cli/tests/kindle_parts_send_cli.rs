//! `mangas kindle --split-mb --send email`: a part the server refuses is reported on its own,
//! the other parts still go out, and the run exits 2.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

mod support;
use support::{noisy_chapter, smtp_refusing};

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

#[test]
fn a_refused_part_is_reported_alone_and_the_others_are_sent() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let chapter = home.join("ch");
    noisy_chapter(&chapter, 6);
    let (port, smtp) = smtp_refusing(3, 2);
    let port = port.to_string();
    let email = [
        "settings",
        "email",
        "--to",
        "reader@kindle.com",
        "--from",
        "me@example.test",
    ];
    let server = ["--host", "127.0.0.1", "--port", &port, "--security", "none"];
    assert!(mangas(home, &[&email[..], &server[..]].concat())
        .status
        .success());
    let book = home.join("book.epub");
    let (chapter, book) = (chapter.to_str().unwrap(), book.to_str().unwrap());
    let args = [
        "kindle",
        chapter,
        "--title",
        "Book",
        "--json",
        "--split-mb",
        "2",
        "--send",
        "email",
        "-o",
        book,
    ];
    let out = mangas(home, &args);

    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files: Vec<&str> = (r["files"].as_array().unwrap().iter())
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(files.len(), 3, "{r}");
    let failed = r["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 1, "{r}");
    assert_eq!(failed[0]["path"], files[1]);
    let reason = failed[0]["reason"].as_str().unwrap();
    assert!(
        reason.contains("book (part 2).epub was written but not sent"),
        "{reason}"
    );
    assert!(reason.contains("mangas send --via email"), "{reason}");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.matches("emailed").count(), 2, "{stderr}");
    assert_eq!(
        stderr.matches("was written but not sent").count(),
        1,
        "{stderr}"
    );
    let log = smtp.join().unwrap();
    assert!(log.contains("Subject: Book (part 3 of 3)"), "{log}");
}
