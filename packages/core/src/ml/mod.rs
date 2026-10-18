//! Optional ONNX detectors (speech bubbles, text, faces) that sharpen smart cutting.
//! Models are downloaded on first use, never bundled. Inference needs the `ml` cargo feature
//! (ONNX Runtime via `ort`); the pure parts (NMS, tiling, row penalties, cache) always build.

pub mod cache;
pub mod nms;
pub mod penalty;
pub mod tiles;

#[cfg(feature = "ml")]
mod detect;
#[cfg(feature = "ml")]
mod preprocess;
#[cfg(feature = "ml")]
mod runtime;
#[cfg(feature = "ml")]
pub use detect::{Detectors, DETECTOR_SCORE_THRESHOLD, NMS_IOU, TILE_OVERLAP, TILE_SIZE};

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::Result;
use crate::smartcut::RowPenalizer;

/// Whether this build can run the detectors at all.
pub const ENABLED: bool = cfg!(feature = "ml");

pub const MODELS: [ModelSpec; 2] = [BUBBLE_TEXT_DETECTOR, FACE_DETECTOR];

/// A downloadable detector. Licences are permissive on purpose (the GPL YOLOv8 bubble model is
/// deliberately not listed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub id: &'static str,
    pub url: &'static str,
    pub licence: &'static str,
    /// Hex SHA-256 of the file at `url`, checked after download.
    pub sha256: Option<&'static str>,
}

pub const BUBBLE_TEXT_DETECTOR: ModelSpec = ModelSpec {
    id: "comic-text-and-bubble-detector",
    url: "https://huggingface.co/ogkalu/comic-text-and-bubble-detector/resolve/main/detector.onnx",
    licence: "Apache-2.0",
    sha256: Some("065744e91c0594ad8663aa8b870ce3fb27222942eded5a3cc388ce23421bd195"),
};

pub const FACE_DETECTOR: ModelSpec = ModelSpec {
    id: "anime-face-detect-v0-n",
    url: "https://huggingface.co/deepghs/anime_face_detection/resolve/main/face_detect_v0_n/model.onnx",
    licence: "MIT",
    sha256: Some("60d40c22cd57a2d129ff913227e7abcb8db9a3b98ebb8ec4fa93a72bee0dcfab"),
};

/// What the desktop/CLI show about the detectors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// Built with the `ml` feature.
    pub enabled: bool,
    /// Every model file is present in the cache.
    pub downloaded: bool,
    pub models: Vec<ModelStatus>,
    /// Execution provider that would be used ("cpu", "coreml", "directml", "cuda"), if enabled.
    pub execution_provider: Option<String>,
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
    Status {
        enabled: ENABLED,
        downloaded: models.iter().all(|m| m.downloaded),
        models,
        execution_provider: execution_provider_name(),
    }
}

#[cfg(feature = "ml")]
fn execution_provider_name() -> Option<String> {
    Some(runtime::provider_name().to_string())
}

#[cfg(not(feature = "ml"))]
fn execution_provider_name() -> Option<String> {
    None
}

/// Downloads the models if needed and returns a smartcut row penalizer backed by them.
#[cfg(feature = "ml")]
pub async fn row_penalizer(
    cache_dir: &Path,
    on_progress: &mut (dyn FnMut(&str, u64, Option<u64>) + Send),
) -> Result<RowPenalizer> {
    let mut paths = Vec::new();
    for spec in &MODELS {
        let mut cb = |done, total| on_progress(spec.id, done, total);
        paths.push(cache::ensure_model(cache_dir, spec, &mut cb).await?);
    }
    let detectors = Detectors::load(&paths[0], &paths[1])?;
    Ok(std::sync::Arc::new(move |img| detectors.row_penalties(img)))
}

#[cfg(not(feature = "ml"))]
pub async fn row_penalizer(
    _cache_dir: &Path,
    _on_progress: &mut (dyn FnMut(&str, u64, Option<u64>) + Send),
) -> Result<RowPenalizer> {
    Err(crate::domain::Error::Other(
        "detectors need a build with the `ml` feature (cargo build --features ml)".into(),
    ))
}
