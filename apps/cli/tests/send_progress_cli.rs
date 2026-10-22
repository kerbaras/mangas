//! The `copying:` progress line of `mangas send --via usb`: ended before an error, and not
//! printed when the book is already on the Kindle.

use std::path::Path;
use std::process::{Command, Output};

fn send_usb(home: &Path, book: &Path, kindle: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args([
            "send",
            book.to_str().unwrap(),
            "--via",
            "usb",
            "--force",
            "--kindle",
        ])
        .arg(kindle)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_book_already_on_the_kindle_prints_no_progress() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    let onboard = kindle.join("documents/Vol 1.epub");
    std::fs::create_dir_all(onboard.parent().unwrap()).unwrap();
    std::fs::write(&onboard, b"epub").unwrap();
    let out = send_usb(home.path(), &onboard, &kindle);
    assert!(out.status.success(), "{out:?}");
    assert!(!stderr(&out).contains("copying:"), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("already on Kindle: "));
}

#[test]
fn a_failed_copy_ends_the_progress_line_before_the_error() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    // A folder where the book should go: the copy runs, then the final rename fails.
    std::fs::create_dir_all(kindle.join("documents/Vol 1.epub/keep")).unwrap();
    let book = home.path().join("Vol 1.epub");
    std::fs::write(&book, b"epub").unwrap();
    let out = send_usb(home.path(), &book, &kindle);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("copying: 0.0/0.0 MB"), "{err}");
    let failure = err.lines().find(|l| l.contains("Couldn't copy to Kindle"));
    let failure = failure.unwrap_or_else(|| panic!("no copy error: {err}"));
    assert!(!failure.contains("copying:"), "{err}");
}
