//! Optional ONNX detectors (speech bubbles, text, faces) that sharpen smart cutting.
//! Models and the ONNX Runtime library are downloaded on demand, never bundled. Inference needs the `ml` cargo feature
//! (ONNX Runtime via `ort`); the pure parts (NMS, tiling, row penalties, cache) always build.

pub mod cache;
pub mod fetch;
pub mod lease;
pub mod models;
pub mod nms;
pub mod penalty;
pub mod runtime;
mod support;
pub mod tiles;

#[cfg(feature = "ml")]
mod detect;
#[cfg(feature = "ml")]
mod preprocess;
#[cfg(feature = "ml")]
mod session;
#[cfg(feature = "ml")]
pub use detect::{Detectors, DETECTOR_SCORE_THRESHOLD, NMS_IOU, TILE_OVERLAP, TILE_SIZE};
pub use fetch::{is_cancelled, CANCELLED};
pub use models::{ModelSpec, BUBBLE_TEXT_DETECTOR, FACE_DETECTOR, MODELS};
pub use runtime::RuntimeStatus;
pub use support::{download, missing_bytes, remove, RUNTIME_ITEM};

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::Result;
use crate::smartcut::RowPenalizer;

/// Whether this build can run the detectors at all.
pub const ENABLED: bool = cfg!(feature = "ml");

/// What the desktop/CLI show about the detectors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// Built with the `ml` feature.
    pub enabled: bool,
    /// The runtime and every model file are present.
    pub downloaded: bool,
    pub runtime: RuntimeStatus,
    pub models: Vec<ModelStatus>,
    /// Execution provider that would be used ("cpu", "coreml", "cuda"), if enabled.
    pub execution_provider: Option<String>,
    /// Bytes a download would fetch now.
    pub download_bytes: u64,
    /// Bytes the runtime and models take on disk.
    pub disk_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelStatus {
    pub id: String,
    pub downloaded: bool,
    pub licence: String,
}

pub fn status(cache_dir: &Path) -> Status {
    let models: Vec<ModelStatus> = MODELS
        .iter()
        .map(|m| ModelStatus {
            id: m.id.to_string(),
            downloaded: cache::is_downloaded(cache_dir, m),
            licence: m.licence.to_string(),
        })
        .collect();
    let runtime = runtime::status(cache_dir);
    let spec = runtime::current().filter(|_| ENABLED);
    Status {
        enabled: ENABLED && spec.is_some(),
        downloaded: models.iter().all(|m| m.downloaded)
            && matches!(runtime, RuntimeStatus::Downloaded { .. }),
        runtime,
        models,
        execution_provider: execution_provider_name(),
        download_bytes: support::missing_bytes(cache_dir, spec.as_ref()),
        disk_bytes: crate::storage::dir_size(cache_dir),
    }
}

/// [`status`] after loading a downloaded runtime, so one that won't load shows as failed with the
/// loader's message. The desktop's detector status and the CLI's `ml status` both ask this.
pub fn checked_status(cache_dir: &Path) -> Status {
    #[cfg(feature = "ml")]
    if matches!(runtime::status(cache_dir), RuntimeStatus::Downloaded { .. }) {
        if let Err(e) = runtime::ensure_loaded(cache_dir) {
            tracing::debug!("onnx runtime check: {e}");
        }
    }
    status(cache_dir)
}

#[cfg(feature = "ml")]
fn execution_provider_name() -> Option<String> {
    Some(session::provider_name().to_string())
}

#[cfg(not(feature = "ml"))]
fn execution_provider_name() -> Option<String> {
    None
}

/// Downloads the runtime and models if needed and returns a smartcut row penalizer backed by them.
#[cfg(feature = "ml")]
pub async fn row_penalizer(
    cache_dir: &Path,
    on_progress: &mut (dyn FnMut(&str, u64, Option<u64>) + Send),
) -> Result<RowPenalizer> {
    let never = std::sync::atomic::AtomicBool::new(false);
    let (paths, _lease) = support::ensure_all(cache_dir, on_progress, &never).await?;
    let detectors = Detectors::load(&paths[0], &paths[1])?;
    Ok(std::sync::Arc::new(move |img| detectors.row_penalties(img)))
}

#[cfg(not(feature = "ml"))]
pub async fn row_penalizer(
    _cache_dir: &Path,
    _on_progress: &mut (dyn FnMut(&str, u64, Option<u64>) + Send),
) -> Result<RowPenalizer> {
    Err(support::not_built())
}

/// The detectors for one cut, or why it runs without them. The default is "detectors off".
#[derive(Clone, Default)]
pub struct Penalizer {
    pub penalizer: Option<RowPenalizer>,
    /// Why the detectors are off, e.g. the runtime's load failure (also in [`status`]).
    pub error: Option<String>,
}

/// [`row_penalizer`], except that a failure (runtime that won't load, failed download, build
/// without `ml`) turns the detectors off instead of stopping the export or preview.
pub async fn penalizer_or_reason(
    cache_dir: &Path,
    on_progress: &mut (dyn FnMut(&str, u64, Option<u64>) + Send),
) -> Penalizer {
    match row_penalizer(cache_dir, on_progress).await {
        Ok(p) => Penalizer {
            penalizer: Some(p),
            error: None,
        },
        Err(e) => {
            tracing::warn!("cutting without detectors: {e}");
            Penalizer {
                penalizer: None,
                error: Some(e.to_string()),
            }
        }
    }
}
