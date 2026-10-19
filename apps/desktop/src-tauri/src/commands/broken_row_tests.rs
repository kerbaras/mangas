//! The real commands with the stored settings row broken. One home for the whole test binary:
//! `XDG_DATA_HOME` is process-wide.

use std::path::PathBuf;
use std::sync::OnceLock;

use mangas_core::library;

use super::export::{export_chapters, ExportEvent};
use super::penalizer::{detector_penalizer, models_dir};
use super::{detectors, open_library, settings_of, settings_or_default};
use crate::paths;

fn broken_home() -> &'static PathBuf {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join(format!("mangas-broken-row-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &home);
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
        home
    })
}

/// The Export dialog's detector status still loads, and the penalizer gets past the settings to
/// the default models folder and the system proxy.
#[tokio::test]
async fn detectors_stay_usable() {
    let home = broken_home();
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
#[tokio::test]
async fn the_export_lands_in_the_default_export_folder() {
    let home = broken_home();
    let chapter = home.join("chapter-1");
    std::fs::create_dir_all(&chapter).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(chapter.join("000.png")).unwrap();
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
