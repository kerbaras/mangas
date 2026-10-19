//! #223: `kindle -o` and `export -o` given a folder (existing, or ending in a separator) write
//! the planned file name inside it, with and without `--replace`; `kindle -o <file>` still names
//! the file.

use std::path::{Path, PathBuf};
use std::process::Command;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;

const ID: &str = "series-1";

/// A home following "Series" with chapter "1" downloaded (one page); returns the chapter folder.
fn home() -> (tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let manga = Manga {
        id: MangaId(ID.into()),
        source: "mangadex".into(),
        title: "Series".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("c1".into()),
        manga_id: manga.id.clone(),
        number: "1".into(),
        language: "en".into(),
        ..Default::default()
    };
    let chapters = std::slice::from_ref(&chapter);
    library::upsert_chapters(&lib, "mangadex", &manga.id, "en", chapters).unwrap();
    let dir = home.path().join("chapters/1");
    std::fs::create_dir_all(&dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("000.png")).unwrap();
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
    (home, dir)
}

/// Runs the CLI in `home`; returns stdout and asserts it exited 0.
fn mangas(home: &Path, args: &[&str]) -> String {
    let run = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(run.status.success(), "{run:?}");
    String::from_utf8_lossy(&run.stdout).into_owned()
}

/// The files in `dir` with their sizes.
fn files(dir: &Path) -> Vec<(String, u64)> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                e.metadata().unwrap().len(),
            )
        })
        .collect();
    names.sort();
    names
}

fn mark(dir: &Path, name: &str) {
    std::fs::write(dir.join(name), "old").unwrap();
}

fn kindle(home: &Path, chapter: &Path, output: &str, extra: &[&str]) -> String {
    let args = ["kindle", chapter.to_str().unwrap(), "-o", output];
    mangas(home, &[&args[..], extra].concat())
}

#[test]
fn kindle_into_an_existing_folder_writes_the_titled_book_inside_it() {
    let (home, chapter) = home();
    let out = home.path().join("kin");
    std::fs::create_dir(&out).unwrap();
    let o = out.to_str().unwrap();
    let first = kindle(home.path(), &chapter, o, &[]);
    assert!(first.starts_with(&format!("{o}/Series.epub: ")), "{first}");
    let second = kindle(home.path(), &chapter, o, &[]);
    assert!(
        second.starts_with(&format!("{o}/Series (2).epub: ")),
        "{second}"
    );
    mark(&out, "Series.epub");
    let replaced = kindle(home.path(), &chapter, o, &["--replace"]);
    assert!(
        replaced.starts_with(&format!("{o}/Series.epub: ")),
        "{replaced}"
    );
    let written = files(&out);
    assert_eq!(
        written.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(),
        ["Series (2).epub", "Series.epub"]
    );
    assert!(written.iter().all(|f| f.1 > 3), "{written:?}");
    assert!(!home.path().join("kin (2)").exists());
}

#[test]
fn kindle_into_a_new_folder_ending_in_a_separator_creates_it() {
    let (home, chapter) = home();
    let first = kindle(home.path(), &chapter, "books/", &["--format", "cbz"]);
    assert!(first.starts_with("books/Series.cbz: "), "{first}");
    let replaced = kindle(
        home.path(),
        &chapter,
        "books/",
        &["--format", "cbz", "--replace"],
    );
    assert!(replaced.starts_with("books/Series.cbz: "), "{replaced}");
    assert_eq!(files(&home.path().join("books")).len(), 1);
}

#[test]
fn kindle_to_a_file_path_still_names_the_file() {
    let (home, chapter) = home();
    let first = kindle(home.path(), &chapter, "book.cbz", &[]);
    assert!(first.starts_with("book.cbz: "), "{first}");
    let second = kindle(home.path(), &chapter, "book.cbz", &[]);
    assert!(second.starts_with("book (2).cbz: "), "{second}");
    mark(home.path(), "book.cbz");
    let replaced = kindle(home.path(), &chapter, "book.cbz", &["--replace"]);
    assert!(replaced.starts_with("book.cbz: "), "{replaced}");
    assert!(
        std::fs::metadata(home.path().join("book.cbz"))
            .unwrap()
            .len()
            > 3
    );
    assert!(home.path().join("book (2).cbz").is_file());
}

fn export(home: &Path, output: &str, extra: &[&str]) -> String {
    let args = ["export", ID, "--format", "epub", "-o", output];
    mangas(home, &[&args[..], extra].concat())
}

#[test]
fn export_into_an_existing_folder_and_a_new_one_ending_in_a_separator() {
    let (home, _) = home();
    let existing = home.path().join("out");
    std::fs::create_dir(&existing).unwrap();
    for (o, dir) in [
        (existing.to_str().unwrap(), existing.clone()),
        ("new/", home.path().join("new")),
    ] {
        let first = export(home.path(), o, &[]);
        let sep = if o.ends_with('/') { "" } else { "/" };
        assert!(
            first.starts_with(&format!("{o}{sep}Series ch 1.epub  (1 chapters")),
            "{first}"
        );
        let second = export(home.path(), o, &[]);
        assert!(
            second.starts_with(&format!("{o}{sep}Series ch 1 (2).epub  (")),
            "{second}"
        );
        mark(&dir, "Series ch 1.epub");
        let replaced = export(home.path(), o, &["--replace"]);
        assert!(
            replaced.starts_with(&format!("{o}{sep}Series ch 1.epub  (")),
            "{replaced}"
        );
        let written = files(&dir);
        assert_eq!(written.len(), 2, "{written:?}");
        assert!(written.iter().all(|f| f.1 > 3), "{written:?}");
    }
}
