//! The real commands with the stored settings row broken.

mod cover;
mod reads;

use std::path::{Path, PathBuf};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::export::volume::Grouping;
use mangas_core::library;

use super::export::{export_chapters, ExportEvent};
use super::penalizer::{detector_penalizer, models_dir};
use super::volumes::export_volumes;
use super::{detectors, open_library, settings_of, settings_or_default};
use crate::paths;

/// All cases run in one test with one temporary home, removed at the end: `XDG_DATA_HOME` and
/// the working directory are process-wide.
#[tokio::test]
async fn broken_settings_row() {
    let home = tempfile::tempdir().unwrap();
    let cwd = std::env::current_dir().unwrap();
    break_settings(home.path());
    detectors_stay_usable(home.path()).await;
    the_export_lands_in_the_default_export_folder(home.path()).await;
    relative_folders_land_under_the_default_export_folder(home.path()).await;
    cover::the_export_embeds_the_cached_series_cover(home.path()).await;
    reads::reads_use_the_defaults_and_writes_refuse(home.path()).await;
    std::env::set_current_dir(cwd).unwrap();
}

fn break_settings(home: &Path) {
    std::env::set_var("XDG_DATA_HOME", home);
    // A dead system proxy: an `ml` build's detector download fails fast through it.
    std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:9");
    let db = paths::library_db().unwrap();
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    drop(library::open(&db).unwrap());
    let broken = "INSERT OR REPLACE INTO setting (key, value) VALUES ('app', '{broken')";
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(broken, [])
        .unwrap();
    assert!(settings_of(&open_library().unwrap()).is_err());
}

/// The Export dialog's detector status still loads, and the penalizer gets past the settings to
/// the default models folder and the system proxy.
async fn detectors_stay_usable(home: &Path) {
    let status = detectors::detectors_status().await.unwrap();
    assert_eq!(models_dir().unwrap(), home.join("mangas/models"));
    let mut missing = false;
    let mut send = |e| missing |= matches!(e, ExportEvent::DetectorsMissing { .. });
    let detection = detector_penalizer(true, &settings_or_default(), &mut send)
        .await
        .unwrap();
    assert!(
        detection.penalizer.is_none(),
        "no detector support in a fresh folder"
    );
    let err = detection.error.unwrap();
    assert!(!err.contains("settings"), "{err}");
    assert_eq!(missing, status.download_bytes > 0);
    assert_eq!(status.enabled, cfg!(feature = "ml"));
}

/// `export_chapters` with a bare file name writes into the default data folder's `kindle`, never
/// into a `~` folder under the working directory.
async fn the_export_lands_in_the_default_export_folder(home: &Path) {
    let chapter = chapter_dir(home, "bare");
    let app = tauri::test::mock_app();
    let book = PathBuf::from("Solo ch 1.epub");
    let stats = export_chapters(
        app.handle().clone(),
        vec![chapter],
        None,
        None,
        None,
        false,
        book,
        None,
        None,
    )
    .await
    .unwrap();
    let expected = home.join("mangas/kindle/Solo ch 1.epub");
    assert_eq!(stats.output, expected);
    assert!(expected.is_file());
    assert!(
        !PathBuf::from("~").exists(),
        "no `~` folder in the working directory"
    );
}

fn chapter_dir(home: &Path, name: &str) -> PathBuf {
    let dir = home.join("chapters").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("000.png")).unwrap();
    dir
}

/// A relative folder typed in the dialog goes under the default export folder in both commands,
/// even when the app was started from `/` by a launcher.
async fn relative_folders_land_under_the_default_export_folder(home: &Path) {
    std::env::set_current_dir("/").unwrap();
    let books = home.join("mangas/kindle/books");
    let app = tauri::test::mock_app();

    let book = PathBuf::from("books/Mixed.epub");
    let dirs = vec![chapter_dir(home, "mixed")];
    let stats = export_chapters(
        app.handle().clone(),
        dirs,
        None,
        None,
        None,
        false,
        book,
        None,
        None,
    );
    assert_eq!(stats.await.unwrap().output, books.join("Mixed.epub"));

    let lib = open_library().unwrap();
    let manga = Manga {
        id: MangaId("solo".into()),
        source: "mangadex".into(),
        title: "Solo".into(),
        ..Default::default()
    };
    let chapter = Chapter {
        id: ChapterId("solo-1".into()),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    library::upsert_chapters(
        &lib,
        "mangadex",
        &manga.id,
        "en",
        std::slice::from_ref(&chapter),
    )
    .unwrap();
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &chapter_dir(home, "solo")).unwrap();
    let volume = |replace: Option<bool>| {
        export_volumes(
            app.handle().clone(),
            "mangadex".into(),
            "solo".into(),
            vec!["solo-1".into()],
            Grouping::Single,
            None,
            None,
            Some(mangas_core::export::Format::Epub),
            None,
            false,
            Some(PathBuf::from("books")),
            replace,
        )
    };
    let written = volume(None).await.unwrap().written;
    assert_eq!(written.len(), 1);
    assert_eq!(written[0].output.parent(), Some(books.as_path()));
    assert!(written[0].output.is_file());

    let first = written[0].output.clone();
    let copy = volume(Some(false)).await.unwrap().written;
    assert_ne!(copy[0].output, first, "replace off writes a numbered copy");
    let replaced = volume(Some(true)).await.unwrap().written;
    assert_eq!(replaced[0].output, first, "replace on writes the same file");
}
