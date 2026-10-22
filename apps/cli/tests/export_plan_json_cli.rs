//! #326: `export --dry-run --json` plans each book against the Send to Kindle budget of its
//! delivery (200 MB, 50 MB for email) with the JPEG quality it would be written at, and writes
//! nothing.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

mod support;
use support::library_with_noisy_chapters;

const ID: &str = "series-1";

fn plan(home: &Path, extra: &[&str]) -> Value {
    let out: Output = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(["export", ID, "--json", "--dry-run", "-o", "/nowhere"])
        .args(extra)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let report: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    report["plan"].clone()
}

#[test]
fn dry_run_json_plans_each_book_against_the_budget_of_its_delivery() {
    let home = tempfile::tempdir().unwrap();
    library_with_noisy_chapters(home.path(), ID);

    let books = plan(home.path(), &[]);
    assert_eq!(books.as_array().map(Vec::len), Some(1), "{books}");
    let book = &books[0];
    assert_eq!(book["budget_bytes"], 200_000_000);
    assert_eq!(book["over_budget"], false);
    assert_eq!(book["jpeg_quality"], 90);
    assert!(book["estimated_bytes"].as_u64().unwrap() > 0, "{book}");
    assert!(!Path::new("/nowhere").exists());

    let book = &plan(home.path(), &["--send", "email", "--quality", "60"])[0];
    assert_eq!(book["budget_bytes"], 50_000_000);
    assert_eq!(book["jpeg_quality"], 60);
}
