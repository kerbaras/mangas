//! #199: `mangas export` with the settings row unreadable (`'{broken'`) still embeds the series
//! cover already cached in the default data folder, like a normal export does.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::covers::{self, CoverCache, Limits};
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library;

const ID: &str = "series-1";
const URL: &str = "https://uploads.mangadex.org/covers/series-1/cover.png";
const COVER_ENTRY: &[u8] = b"OEBPS/images/cover.jpg";

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        // A dead system proxy: the cover can only come from the cache.
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

/// A followed manga with a downloaded chapter and its cover cached in the default data folder.
fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let app = home.path().join("data/mangas");
    std::fs::create_dir_all(&app).unwrap();
    let lib = library::open(&app.join("mangas.db")).unwrap();
    let manga = Manga {
        id: MangaId(ID.into()),
        source: "mangadex".into(),
        title: "Series".into(),
        cover_url: Some(URL.into()),
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
    let cache = CoverCache::new(covers::dir(&app), Limits::default());
    cache.store("mangadex", ID, URL, &red_png()).unwrap();
    home
}

fn red_png() -> Vec<u8> {
    let cover = image::RgbImage::from_pixel(60, 90, image::Rgb([200, 30, 30]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    cover.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

fn break_settings(home: &Path) {
    let broken = "INSERT OR REPLACE INTO setting (key, value) VALUES ('app', '{broken')";
    rusqlite::Connection::open(home.join("data/mangas/mangas.db"))
        .unwrap()
        .execute(broken, [])
        .unwrap();
}

/// Exports into `books-<name>` and says whether the EPUB holds the series cover.
fn export_has_cover(home: &Path, name: &str) -> bool {
    let dir = home.join(format!("books-{name}"));
    let out = mangas(home, &["export", ID, "-o", dir.to_str().unwrap()]);
    assert!(out.status.success(), "{out:?}");
    let book = std::fs::read(dir.join("Series ch 1.epub")).unwrap();
    book.windows(COVER_ENTRY.len()).any(|w| w == COVER_ENTRY)
}

#[test]
fn the_series_cover_survives_unreadable_settings() {
    let home = home();
    assert!(export_has_cover(home.path(), "normal"), "normal export");
    break_settings(home.path());
    let lib = library::open(&home.path().join("data/mangas/mangas.db")).unwrap();
    assert!(
        library::get_settings(&lib).is_err(),
        "the row is unreadable"
    );
    assert!(export_has_cover(home.path(), "fallback"), "fallback export");
}
