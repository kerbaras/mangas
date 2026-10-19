//! `replace` through the real export path, for every format: off keeps the old file and writes
//! ` (2)` next to it; on writes the same path with the new book; a write that fails half way
//! (the child process hits its file size limit) leaves the old file as it was.

mod export_support;

use std::path::{Path, PathBuf};

use export_support::chapter;
use mangas_core::export::kindle::{export, ExportRequest};
use mangas_core::export::{profile, Format};
use mangas_core::smartcut::Preset;

const OLD: &[u8] = b"the old book";
const CHILD_ENV: &str = "MANGAS_REPLACE_CHILD";

fn request(root: &Path, format: Format, replace: bool) -> ExportRequest {
    ExportRequest {
        chapter_dirs: vec![root.join("Chapter 1"), root.join("Chapter 2")],
        device: profile("kindle-paperwhite12").unwrap(),
        format,
        cut: Preset::Balanced.options(),
        page: Default::default(),
        title: "Book".into(),
        title_explicit: false,
        output: root.join(format!("book.{format}")),
        replace,
        meta: Default::default(),
        labels: Default::default(),
    }
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    chapter(root.path(), "Chapter 1", 2);
    chapter(root.path(), "Chapter 2", 3);
    root
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with("Chapter"))
        .collect();
    names.sort();
    names
}

fn magic(format: Format) -> &'static [u8] {
    match format {
        Format::Pdf => b"%PDF",
        Format::Epub | Format::Cbz => b"PK",
    }
}

#[test]
fn off_writes_a_numbered_copy_and_on_replaces_the_file() {
    for format in Format::ALL {
        let root = fixture();
        let target = root.path().join(format!("book.{format}"));
        std::fs::write(&target, OLD).unwrap();

        let kept = export(&request(root.path(), format, false), None, &mut |_| {}).unwrap();
        let copy = root.path().join(format!("book (2).{format}"));
        assert_eq!(kept.output, copy, "{format}");
        assert_eq!(std::fs::read(&target).unwrap(), OLD, "{format}");

        let replaced = export(&request(root.path(), format, true), None, &mut |_| {}).unwrap();
        assert_eq!(replaced.output, target, "{format}");
        let new = std::fs::read(&target).unwrap();
        assert!(new.starts_with(magic(format)), "{format}: a new book");
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions();
        assert_eq!(
            mode(&target),
            mode(&copy),
            "{format}: same mode as a new export"
        );
        let expected = vec![format!("book (2).{format}"), format!("book.{format}")];
        assert_eq!(names(root.path()), expected, "no (3), no temporary file");
    }
}

#[test]
fn replacing_into_a_missing_folder_creates_it() {
    let root = fixture();
    let req = ExportRequest {
        output: root.path().join("new/book.cbz"),
        ..request(root.path(), Format::Cbz, true)
    };
    let stats = export(&req, None, &mut |_| {}).unwrap();
    assert_eq!(stats.output, root.path().join("new/book.cbz"));
}

/// Child half of the failure test: replaces every format's book in the folder given by the env
/// var, under a file size limit far below a book, so each write fails part way.
#[test]
fn child_replaces_under_a_size_limit() {
    let Ok(root) = std::env::var(CHILD_ENV) else {
        return;
    };
    for format in Format::ALL {
        let failed = export(&request(Path::new(&root), format, true), None, &mut |_| {});
        assert!(failed.is_err(), "{format}: {failed:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_failed_replace_keeps_the_old_file() {
    let root = fixture();
    let targets: Vec<PathBuf> = Format::ALL
        .iter()
        .map(|f| root.path().join(format!("book.{f}")))
        .collect();
    for target in &targets {
        std::fs::write(target, OLD).unwrap();
    }
    // `ulimit -f 4`: writes past a few KB fail with EFBIG (SIGXFSZ ignored) mid-book. The
    // child's output is piped: a redirected stdout file would hit the limit too.
    let script = r#"trap "" XFSZ; ulimit -f 4; exec "$0" "$@""#;
    let out = std::process::Command::new("sh")
        .args(["-c", script])
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "child_replaces_under_a_size_limit",
            "--test-threads=1",
        ])
        .env(CHILD_ENV, root.path())
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "every write failed in the child: {log}"
    );
    for target in &targets {
        assert_eq!(std::fs::read(target).unwrap(), OLD, "{}", target.display());
    }
    let mut expected: Vec<String> = Format::ALL.iter().map(|f| format!("book.{f}")).collect();
    expected.sort();
    assert_eq!(names(root.path()), expected, "no temporary file left");
}
