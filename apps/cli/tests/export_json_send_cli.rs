//! #218: with `--send`, stdout stays the one JSON object (where each book went is on stderr), and
//! a volume that fails to export after others were written still prints the report. #220: the
//! run stops there, the books before it stay, nothing of it is left and the rest are listed.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;
use serde_json::Value;

const ID: &str = "series-1";

/// Follows a manga with chapters "1", "2"… downloaded; `pages[i]` pages in chapter i+1.
fn library_with(home: &Path, pages: &[usize]) {
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
    let numbers: Vec<String> = (1..=pages.len()).map(|n| n.to_string()).collect();
    let chapters: Vec<Chapter> = numbers.iter().map(|n| chapter(n)).collect();
    library::upsert_chapters(&lib, "mangadex", &manga.id, "en", &chapters).unwrap();
    for (c, &count) in chapters.iter().zip(pages) {
        let dir = home.join("chapters").join(&c.number);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..count {
            let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
            page.save(dir.join(format!("{i:03}.png"))).unwrap();
        }
        library::mark_downloaded(&lib, "mangadex", &c.id, &dir).unwrap();
    }
}

/// `mangas export <ID> --every 1 -o out --json --send usb` to a fake Kindle mount.
fn export_to_kindle(home: &Path) -> Output {
    let kindle = home.join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let out = home.join("out");
    let args = [
        "export", ID, "--every", "1", "--json", "--send", "usb", "--force", "-o",
    ];
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .arg(&out)
        .arg("--kindle")
        .arg(&kindle)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn report(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

#[test]
fn sent_books_keep_stdout_one_json_object() {
    let home = tempfile::tempdir().unwrap();
    library_with(home.path(), &[2, 2]);
    let out = export_to_kindle(home.path());
    assert!(out.status.success(), "{out:?}");
    let r = report(&out);
    assert_eq!(r["files"].as_array().unwrap().len(), 2, "{r}");
    assert_eq!(r["failed"], serde_json::json!([]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.matches("copied to").count(), 2, "{stderr}");

    let kindle = home.path().join("kindle-run");
    std::fs::create_dir_all(&kindle).unwrap();
    let book = kindle.join("book.epub");
    let chapter = home.path().join("chapters/1");
    let out = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args([
            "kindle",
            chapter.to_str().unwrap(),
            "--json",
            "--send",
            "usb",
            "--force",
        ])
        .arg("-o")
        .arg(&book)
        .arg("--kindle")
        .arg(home.path().join("Kindle"))
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(report(&out)["files"][0]["path"], book.to_str().unwrap());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("copied to"),
        "{out:?}"
    );
}

#[test]
fn a_later_volume_failing_still_reports_the_books_written() {
    let home = tempfile::tempdir().unwrap();
    library_with(home.path(), &[2, 0]);
    let out = export_to_kindle(home.path());
    assert!(!out.status.success(), "{out:?}");
    assert_ne!(
        out.status.code(),
        Some(2),
        "an export error, not a send error"
    );
    let r = report(&out);
    let dir = home.path().join("out");
    let (first, second) = (dir.join("Series v1.epub"), dir.join("Series v2.epub"));
    assert_eq!(r["files"][0]["path"], first.to_str().unwrap());
    assert_eq!(r["failed"][0]["path"], second.to_str().unwrap());
    assert!(first.is_file() && !second.exists());
}

#[test]
fn a_failed_volume_leaves_nothing_and_the_books_after_it_are_listed_not_written() {
    let home = tempfile::tempdir().unwrap();
    library_with(home.path(), &[2, 0, 2]);
    let out = export_to_kindle(home.path());
    assert!(!out.status.success(), "{out:?}");
    let r = report(&out);
    let dir = home.path().join("out");
    let [first, second, third] = ["v1", "v2", "v3"].map(|v| dir.join(format!("Series {v}.epub")));
    assert_eq!(r["files"].as_array().unwrap().len(), 1, "{r}");
    assert_eq!(r["files"][0]["path"], first.to_str().unwrap());
    assert_eq!(r["failed"][0]["path"], second.to_str().unwrap());
    assert_eq!(r["not_written"][0]["path"], third.to_str().unwrap());
    assert_eq!(
        r["not_written"][0]["reason"],
        "not started: an earlier book failed"
    );
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        left,
        [first.file_name().unwrap()],
        "only the complete book stays"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Series v2 failed: ")
            && err.contains("the 1 book written before it is complete and kept")
            && err.contains("not written: Series v3"),
        "{err}"
    );
}

#[test]
fn a_failed_volume_whose_name_is_taken_reports_the_numbered_name_it_would_have_taken() {
    let home = tempfile::tempdir().unwrap();
    library_with(home.path(), &[2, 0, 2]);
    let dir = home.path().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Series v2.epub"), b"old").unwrap();
    let out = export_to_kindle(home.path());
    assert!(!out.status.success(), "{out:?}");
    let r = report(&out);
    let path = |name: &str| dir.join(name).to_str().unwrap().to_string();
    assert_eq!(r["failed"][0]["path"], path("Series v2 (2).epub"), "{r}");
    assert_eq!(r["not_written"][0]["path"], path("Series v3.epub"), "{r}");
    assert_eq!(std::fs::read(dir.join("Series v2.epub")).unwrap(), b"old");
}
