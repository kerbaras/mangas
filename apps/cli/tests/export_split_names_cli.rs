//! #134: with `--split-mb`, the dry-run line names the first part each book is written under,
//! also when two books share a planned name (`Book (part 1)`, `Book (2) (part 1)`) and when an
//! earlier set of parts already holds a book's name.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

mod support;
use support::library_with_noisy_chapters;

const ID: &str = "series-1";

/// `mangas export --every 1 --name Book --split-mb 1` (plus `extra`) into `home/out`.
fn export(home: &Path, extra: &[&str]) -> String {
    let args = [
        "export",
        ID,
        "--every",
        "1",
        "--name",
        "Book",
        "--split-mb",
        "1",
    ];
    let run = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .args(extra)
        .arg("-o")
        .arg(home.join("out"))
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(run.status.success(), "{run:?}");
    String::from_utf8_lossy(&run.stdout).into_owned()
}

/// The first-part paths the plan lines (`…, in parts of 1 MB if bigger: <path>, …)`) announce.
fn announced(stdout: &str) -> Vec<PathBuf> {
    stdout
        .lines()
        .filter_map(|l| l.split_once(" if bigger: "))
        .map(|(_, rest)| PathBuf::from(rest.trim_end_matches(", …)")))
        .collect()
}

/// The part 1 files a `--json` report lists, in book order.
fn written_first_parts(report: &str) -> Vec<PathBuf> {
    let r: Value = serde_json::from_str(report).unwrap_or_else(|e| panic!("{e}: {report}"));
    r["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["part"]["index"] == 1)
        .map(|f| PathBuf::from(f["path"].as_str().unwrap()))
        .collect()
}

fn check(home: &Path, expected: [&str; 2]) {
    let dry = export(home, &["--dry-run"]);
    let shown = announced(&dry);
    let out = home.join("out");
    let names: Vec<PathBuf> = expected.iter().map(|n| out.join(n)).collect();
    assert_eq!(shown, names, "{dry}");
    let written = written_first_parts(&export(home, &["--json"]));
    assert_eq!(written, shown, "the dry run names the files written");
    assert!(written.iter().all(|p| p.exists()));
}

#[test]
fn the_dry_run_names_each_first_part_of_books_sharing_a_name() {
    let home = tempfile::tempdir().unwrap();
    library_with_noisy_chapters(home.path(), ID);
    check(
        home.path(),
        ["Book (part 1).epub", "Book (2) (part 1).epub"],
    );
}

#[test]
fn the_dry_run_skips_parts_already_in_the_folder() {
    let home = tempfile::tempdir().unwrap();
    library_with_noisy_chapters(home.path(), ID);
    let out = home.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("Book (2) (part 1).epub"), "old").unwrap();
    check(
        home.path(),
        ["Book (part 1).epub", "Book (3) (part 1).epub"],
    );
}

#[test]
fn an_earlier_set_of_parts_takes_the_book_name() {
    let home = tempfile::tempdir().unwrap();
    library_with_noisy_chapters(home.path(), ID);
    let out = home.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    for n in 1..=2 {
        std::fs::write(out.join(format!("Book (part {n}).epub")), "old").unwrap();
    }
    check(
        home.path(),
        ["Book (2) (part 1).epub", "Book (3) (part 1).epub"],
    );
    for n in 1..=2 {
        let old = std::fs::read(out.join(format!("Book (part {n}).epub"))).unwrap();
        assert_eq!(old, b"old");
    }
}
