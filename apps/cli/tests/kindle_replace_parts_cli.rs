//! #134: `kindle --replace` leaves exactly the new set: fewer parts remove the old set's higher
//! parts, a split removes the whole book, one file removes every part. Other files stay.

use std::path::Path;
use std::process::{Command, Output};

mod support;
use support::noisy_chapter;

fn kindle(home: &Path, chapter: &Path, book: &Path, split_mb: Option<&str>) {
    let out = run(home, chapter, book, split_mb, true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
}

fn run(home: &Path, chapter: &Path, book: &Path, split_mb: Option<&str>, replace: bool) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mangas-cli"));
    cmd.args(["kindle", chapter.to_str().unwrap(), "-o"])
        .arg(book)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG");
    if replace {
        cmd.arg("--replace");
    }
    if let Some(mb) = split_mb {
        cmd.args(["--split-mb", mb]);
    }
    cmd.output().unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

const OTHERS: [&str; 3] = ["book (2) (part 4).epub", "book (part 4).mobi", "other.epub"];

fn with_others(mut book: Vec<String>) -> Vec<String> {
    book.extend(OTHERS.map(String::from));
    book.sort();
    book
}

fn parts(n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("book (part {i}).epub")).collect()
}

#[test]
fn a_replacement_leaves_only_the_new_set_of_the_same_book() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("chapters/1");
    noisy_chapter(&chapter, 8);
    let out = home.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    for other in OTHERS {
        std::fs::write(out.join(other), b"not this book").unwrap();
    }
    let book = out.join("book.epub");

    kindle(home.path(), &chapter, &book, Some("1"));
    let many = names(&out).len() - OTHERS.len();
    assert_eq!(names(&out), with_others(parts(many)));

    kindle(home.path(), &chapter, &book, Some("2"));
    let fewer = names(&out).len() - OTHERS.len();
    assert!(1 < fewer && fewer < many, "{many} then {fewer} parts");
    assert_eq!(names(&out), with_others(parts(fewer)));

    kindle(home.path(), &chapter, &book, None);
    assert_eq!(names(&out), with_others(vec!["book.epub".into()]));

    kindle(home.path(), &chapter, &book, Some("2"));
    assert_eq!(names(&out), with_others(parts(fewer)));

    kindle(home.path(), &chapter, &book, Some("1"));
    assert_eq!(names(&out), with_others(parts(many)));
    for other in OTHERS {
        assert_eq!(std::fs::read(out.join(other)).unwrap(), b"not this book");
    }
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_listed_stops_every_export_before_it_writes() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("chapters/1");
    noisy_chapter(&chapter, 4);
    let out = home.path().join("out");
    let book = out.join("book.epub");
    kindle(home.path(), &chapter, &book, Some("1"));
    let before = names(&out);
    let bytes: Vec<Vec<u8>> = before
        .iter()
        .map(|n| std::fs::read(out.join(n)).unwrap())
        .collect();
    let mode = |m| std::fs::set_permissions(&out, std::fs::Permissions::from_mode(m)).unwrap();
    mode(0o300);
    if std::fs::read_dir(&out).is_ok() {
        mode(0o755);
        eprintln!("skipped: running as root, -wx folders can still be listed");
        return;
    }
    // A split or a replacement must see the folder; a plain export only claims its own name.
    let runs = [(Some("2"), true), (Some("2"), false), (None, true)];
    let failed: Vec<Output> = runs
        .iter()
        .map(|&(mb, replace)| run(home.path(), &chapter, &book, mb, replace))
        .collect();
    mode(0o755);
    for (out_, (mb, replace)) in failed.iter().zip(runs) {
        let stderr = String::from_utf8_lossy(&out_.stderr);
        let case = format!("split {mb:?}, replace {replace}: {stderr}");
        assert_eq!(out_.status.code(), Some(1), "{case}");
        assert!(
            stderr.contains(&format!("couldn't list {}: ", out.display())),
            "{case}"
        );
    }
    assert_eq!(names(&out), before);
    let after: Vec<Vec<u8>> = before
        .iter()
        .map(|n| std::fs::read(out.join(n)).unwrap())
        .collect();
    assert!(after == bytes, "the old parts changed");
    mode(0o300);
    let plain = run(home.path(), &chapter, &book, None, false);
    mode(0o755);
    let stderr = String::from_utf8_lossy(&plain.stderr);
    assert!(plain.status.success(), "plain export: {stderr}");
    assert!(book.is_file(), "the plain export writes the file");
    let mut with_book = before.clone();
    with_book.push("book.epub".into());
    with_book.sort();
    assert_eq!(names(&out), with_book);
}
