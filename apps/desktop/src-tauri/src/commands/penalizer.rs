//! The detector row penalizer for exports and previews. A broken settings row never blocks it:
//! detector support then lives in the default data folder and downloads use the system proxy.

use std::path::PathBuf;

use mangas_core::ml;
use tauri::AppHandle;

use super::export::{emit, ExportEvent};
use super::settings_or_default;
use crate::paths;

pub(super) fn models_dir() -> Result<PathBuf, String> {
    paths::models(&settings_or_default())
}

/// The detector row penalizer when `detectors` is on, reporting runtime/model downloads as events.
/// Detectors that can't run leave it off with the reason; the cut goes on without them.
pub(super) async fn penalizer(app: &AppHandle, detectors: bool) -> Result<ml::Penalizer, String> {
    detector_penalizer(detectors, &mut |event| emit(app, event)).await
}

async fn detector_penalizer(
    detectors: bool,
    send: &mut (dyn FnMut(ExportEvent) + Send),
) -> Result<ml::Penalizer, String> {
    if !detectors {
        return Ok(ml::Penalizer::default());
    }
    let settings = settings_or_default();
    let cache = paths::models(&settings)?;
    let bytes = ml::status(&cache).download_bytes;
    if bytes > 0 {
        send(ExportEvent::DetectorsMissing { bytes });
    }
    let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
        let id = id.to_string();
        send(ExportEvent::ModelDownload { id, done, total });
    };
    Ok(ml::penalizer_or_reason(&cache, &settings.network(), &mut on_progress).await)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use mangas_core::library;

    use super::*;
    use crate::commands::{detectors, open_library, settings_of};

    /// The export step with the stored settings row broken: the Export dialog's detector status
    /// still loads, and the penalizer gets past the settings to the default models folder and
    /// the system proxy (a dead one here, so an `ml` build's download fails fast through it).
    #[tokio::test]
    async fn unreadable_settings_leave_detectors_usable() {
        let home = std::env::temp_dir().join(format!("mangas-penalizer-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &home);
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

        let status = detectors::detectors_status().await.unwrap();
        assert_eq!(models_dir().unwrap(), home.join("mangas/models"));
        let mut missing = false;
        let mut send = |e| missing |= matches!(e, ExportEvent::DetectorsMissing { .. });
        let detection = detector_penalizer(true, &mut send).await.unwrap();
        assert!(
            detection.penalizer.is_none(),
            "no detector support in a fresh folder"
        );
        let err = detection.error.unwrap();
        assert!(!err.contains("settings"), "{err}");
        assert_eq!(missing, status.download_bytes > 0);
        assert_eq!(status.enabled, cfg!(feature = "ml"));
        std::fs::remove_dir_all(&home).ok();
    }
}
