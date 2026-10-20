//! A full disk mid-export: a plain "not enough space in <folder>" and nothing left behind (no
//! book, no `.part`, no spool). The disk is "filled" with a file size limit (`ulimit -f`) in a
//! child run of this test, so every write past a few KB fails as a full disk would.
#![cfg(unix)]

mod export_support;

use std::path::Path;
use std::process::Command;

use export_support::chapter;
use mangas_core::export::kindle::{export, ExportRequest};
use mangas_core::export::{profile, Format};
use mangas_core::smartcut::Preset;

const CHILD: &str = "MANGAS_NO_SPACE_CHILD";

fn request(root: &Path, out: &Path, format: Format) -> ExportRequest {
    ExportRequest {
        chapter_dirs: vec![root.join("Chapter 1")],
        device: profile("kindle-paperwhite12").unwrap(),
        format,
        cut: Preset::Balanced.options(),
        page: Default::default(),
        title: "Book".into(),
        title_explicit: false,
        output: out.join(format!("Book.{format}")),
        replace: false,
        split: None,
        meta: Default::default(),
        labels: Default::default(),
    }
}

/// In the child: export every format into `$CHILD/out` and print each error.
fn child(root: &Path) {
    for format in [Format::Epub, Format::Cbz, Format::Pdf] {
        let req = request(root, &root.join("out"), format);
        match export(&req, None, &mut |_| {}) {
            Ok(stats) => println!("{format}: wrote {}", stats.output.display()),
            Err(e) => println!("{format}: {e}"),
        }
    }
}

#[test]
fn a_full_disk_fails_plainly_and_leaves_nothing() {
    if let Some(root) = std::env::var_os(CHILD) {
        return child(Path::new(&root));
    }
    let root = tempfile::tempdir().unwrap();
    chapter(root.path(), "Chapter 1", 3);
    let exe = std::env::current_exe().unwrap();
    let output = Command::new("sh")
        .args(["-c", r#"trap "" XFSZ; ulimit -f 8; exec "$0" "$@""#])
        .arg(exe)
        .args(["a_full_disk_fails_plainly_and_leaves_nothing", "--exact"])
        .args(["--nocapture", "--test-threads=1"])
        .env(CHILD, root.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out = root.path().join("out");
    let expected = format!("not enough space in {}", out.display());
    for format in ["epub", "cbz", "pdf"] {
        assert!(
            stdout.contains(&format!("{format}: {expected}")),
            "{format}: {stdout}"
        );
    }
    let left: Vec<_> = std::fs::read_dir(&out).unwrap().collect();
    assert!(left.is_empty(), "{left:?}");
}
