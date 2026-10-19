//! The detector row penalizer for exports and previews. A broken settings row never blocks it:
//! detector support then lives in the default data folder and downloads use the system proxy.

use std::path::PathBuf;

use mangas_core::library::Settings;
use mangas_core::ml;
use tauri::{AppHandle, Runtime};

use super::export::{emit, ExportEvent};
use super::settings_or_default;
use crate::paths;

pub(super) fn models_dir() -> Result<PathBuf, String> {
    paths::models(&settings_or_default())
}

/// The detector row penalizer when `detectors` is on, reporting runtime/model downloads as events.
/// Detectors that can't run leave it off with the reason; the cut goes on without them.
pub(super) async fn penalizer<R: Runtime>(
    app: &AppHandle<R>,
    detectors: bool,
    settings: &Settings,
) -> Result<ml::Penalizer, String> {
    detector_penalizer(detectors, settings, &mut |event| emit(app, event)).await
}

pub(super) async fn detector_penalizer(
    detectors: bool,
    settings: &Settings,
    send: &mut (dyn FnMut(ExportEvent) + Send),
) -> Result<ml::Penalizer, String> {
    if !detectors {
        return Ok(ml::Penalizer::default());
    }
    let cache = paths::models(settings)?;
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
