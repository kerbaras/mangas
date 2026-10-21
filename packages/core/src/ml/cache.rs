//! Download-and-cache of detector models into the app data directory, with SHA-256 checks.

use std::path::{Path, PathBuf};

use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};

use super::fetch::fetch;
use super::ModelSpec;
use crate::domain::IoAt;
use crate::domain::{Error, Result};
use crate::library::Network;

pub fn model_path(cache_dir: &Path, spec: &ModelSpec) -> PathBuf {
    cache_dir.join(format!("{}.onnx", spec.id))
}

pub fn is_downloaded(cache_dir: &Path, spec: &ModelSpec) -> bool {
    model_path(cache_dir, spec).is_file()
}

/// Where a download is written before it is renamed into place.
pub fn part_path(path: &Path) -> PathBuf {
    path.with_extension("onnx.part")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn verify(spec: &ModelSpec, bytes: &[u8]) -> Result<()> {
    match spec.sha256 {
        Some(want) if sha256_hex(bytes) != want => Err(Error::Other(format!(
            "model {}: sha256 mismatch (expected {want})",
            spec.id
        ))),
        _ => Ok(()),
    }
}

/// Returns the local path of `spec`, downloading it first if missing. `on_progress` receives
/// (downloaded bytes, total bytes if known); setting `cancel` stops the download.
pub async fn ensure_model(
    cache_dir: &Path,
    spec: &ModelSpec,
    network: &Network,
    on_progress: &mut (dyn FnMut(u64, Option<u64>) + Send),
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let path = model_path(cache_dir, spec);
    if path.is_file() {
        if verify(spec, &std::fs::read(&path).at(&path)?).is_ok() {
            return Ok(path);
        }
        log::warn!(
            "model {}: cached file failed sha256 check, downloading again",
            spec.id
        );
        std::fs::remove_file(&path).at(&path)?;
    }
    std::fs::create_dir_all(cache_dir).at(cache_dir)?;
    let bytes = fetch(spec.url, network, on_progress, cancel).await?;
    verify(spec, &bytes)?;
    let tmp = part_path(&path);
    std::fs::write(&tmp, &bytes).at(&tmp)?;
    std::fs::rename(&tmp, &path).at(&path)?;
    Ok(path)
}
