//! #239: `mangas kindle` on a chapter folder that is not the one the library stored (copied or
//! moved) still names the book after the library title, never the manga id.

use std::path::{Path, PathBuf};
use std::process::Command;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;

/// A library following "Kaiju No. 8" (`mangadex/m-uuid`) with chapter 1 stored at
/// `<home>/data/mangas/chapters/mangadex/m-uuid/1`.
fn follow(home: &Path) {
    std::fs::create_dir_all(home.join("data/mangas")).unwrap();
    let lib = library::open(&home.join("data/mangas/mangas.db")).unwrap();
    let id = MangaId("m-uuid".into());
    let manga = Manga {
        id: id.clone(),
        source: "mangadex".into(),
        title: "Kaiju No. 8".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("c1".into()),
        manga_id: id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    library::upsert_chapters(&lib, "mangadex", &id, "en", std::slice::from_ref(&chapter)).unwrap();
    let stored = home.join("data/mangas/chapters/mangadex/m-uuid/1");
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &stored).unwrap();
}

/// A one-page chapter folder at `dir`.
fn chapter(dir: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("001.png")).unwrap();
    dir
}

/// Runs `mangas kindle <dir> -o <home>/out/` and returns the book's file name.
fn kindle(home: &Path, dir: &Path) -> String {
    let out = home.join("out");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .arg("kindle")
        .arg(dir)
        .arg("-o")
        .arg(format!("{}/", out.display()))
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(run.status.success(), "{run:?}");
    let books: Vec<PathBuf> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(books.len(), 1, "{books:?}");
    books[0].file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn a_moved_chapter_folder_is_named_after_the_library_title() {
    let home = tempfile::tempdir().unwrap();
    follow(home.path());
    let moved = chapter(home.path().join("moved/mangadex/m-uuid/1"));
    assert_eq!(kindle(home.path(), &moved), "Kaiju No. 8.epub");
    let old_data = chapter(home.path().join("old-data/m-uuid/1"));
    assert_eq!(kindle(home.path(), &old_data), "Kaiju No. 8.epub");
}

#[test]
fn a_download_folder_the_library_does_not_know_is_not_named_after_its_id() {
    let home = tempfile::tempdir().unwrap();
    follow(home.path());
    let unknown = chapter(home.path().join("other/mangadex/1d7c44c5-ab12/2"));
    let name = kindle(home.path(), &unknown);
    assert!(!name.contains("1d7c44c5"), "{name}");
    assert_eq!(name, "Manga.epub");
    let named = chapter(home.path().join("Kaiju No. 8 scans/2"));
    assert_eq!(kindle(home.path(), &named), "Kaiju No. 8 scans.epub");
}
