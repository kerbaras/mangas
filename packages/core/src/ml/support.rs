//! "Detector support" as one unit: the ONNX Runtime library plus the models, downloaded,
//! sized and removed together.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use super::cache::{ensure_model, is_downloaded, model_path, part_path};
use super::lease::{self, Lease, Purpose};
use super::{runtime, status, Status, ENABLED, MODELS};
use crate::domain::{Error, Result};
use crate::library::Network;

/// Progress item name of the runtime download (models report their id).
pub const RUNTIME_ITEM: &str = "onnxruntime";

pub(super) fn not_built() -> Error {
    Error::Other("detectors need a build with the `ml` feature (cargo build --features ml)".into())
}

/// Downloads whatever is missing (runtime first, then each model) and loads the runtime.
/// `on_progress` gets (item, bytes so far, total if known).
pub async fn download(
    models_dir: &Path,
    network: &Network,
    on_progress: &mut (dyn FnMut(&str, u64, Option<u64>) + Send),
    cancel: &AtomicBool,
) -> Result<Status> {
    ensure_all(models_dir, network, on_progress, cancel).await?;
    Ok(status(models_dir))
}

/// Model paths in [`MODELS`] order, everything present and the runtime loaded; keep the lease
/// until the models are loaded.
pub(super) async fn ensure_all(
    models_dir: &Path,
    network: &Network,
    on_progress: &mut (dyn FnMut(&str, u64, Option<u64>) + Send),
    cancel: &AtomicBool,
) -> Result<(Vec<PathBuf>, Lease)> {
    if !ENABLED {
        return Err(not_built());
    }
    let spec = runtime::current().ok_or_else(runtime::unsupported)?;
    // Exclusive while something downloads; shared while the files are only read, so exports
    // run side by side but never under a removal or move.
    let lease = if missing_bytes(models_dir, Some(&spec)) > 0 {
        lease::acquire(models_dir, Purpose::Download)?
    } else {
        lease::share(models_dir)?
    };
    let mut on_runtime = |done, total| on_progress(RUNTIME_ITEM, done, total);
    runtime::ensure(models_dir, &spec, network, &mut on_runtime, cancel).await?;
    let mut paths = Vec::new();
    for model in &MODELS {
        let mut on_model = |done, total| on_progress(model.id, done, total);
        paths.push(ensure_model(models_dir, model, network, &mut on_model, cancel).await?);
    }
    #[cfg(feature = "ml")]
    runtime::ensure_loaded(models_dir)?;
    // Only a complete download replaces a pending removal; a cancelled one keeps Remove again.
    runtime::clear_pending(models_dir)?;
    Ok((paths, lease))
}

/// Bytes still to download: the runtime archive if the library is missing, plus missing models.
pub fn missing_bytes(models_dir: &Path, spec: Option<&runtime::RuntimeSpec>) -> u64 {
    let runtime = spec
        .filter(|s| !runtime::lib_path(models_dir, s).is_file())
        .map_or(0, |s| s.size);
    let models: u64 = MODELS
        .iter()
        .filter(|m| !is_downloaded(models_dir, m))
        .map(|m| m.size)
        .sum();
    runtime + models
}

/// Deletes the models and the runtime; returns the bytes freed. A runtime the OS keeps locked
/// (a loaded DLL on Windows) stays, and status reports it as `RemovePending` until a later remove.
pub fn remove(models_dir: &Path) -> Result<u64> {
    let _lease = lease::acquire(models_dir, Purpose::Remove)?;
    let mut freed = 0;
    for model in &MODELS {
        let path = model_path(models_dir, model);
        for file in [part_path(&path), path] {
            if let Ok(meta) = std::fs::metadata(&file) {
                freed += meta.len();
                std::fs::remove_file(&file)?;
            }
        }
    }
    let root = runtime::root(models_dir);
    if root.exists() {
        let size = crate::storage::dir_size(&root);
        match std::fs::remove_dir_all(&root) {
            Ok(()) => freed += size,
            Err(e) => {
                log::warn!("ONNX Runtime still in use, removal pending: {e}");
                std::fs::write(runtime::pending_marker(models_dir), e.to_string())?;
                return Ok(freed + size.saturating_sub(crate::storage::dir_size(&root)));
            }
        }
    }
    runtime::clear_pending(models_dir)?;
    runtime::forget_failure();
    Ok(freed)
}
