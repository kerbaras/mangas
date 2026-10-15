//! Download-and-cache of detector models into the app data directory, with SHA-256 checks.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};

use super::ModelSpec;
use crate::domain::{Error, Result};

pub fn model_path(cache_dir: &Path, spec: &ModelSpec) -> PathBuf {
    cache_dir.join(format!("{}.onnx", spec.id))
}

pub fn is_downloaded(cache_dir: &Path, spec: &ModelSpec) -> bool {
    model_path(cache_dir, spec).is_file()
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
/// (downloaded bytes, total bytes if known).
pub async fn ensure_model(
    cache_dir: &Path,
    spec: &ModelSpec,
    on_progress: &mut (dyn FnMut(u64, Option<u64>) + Send),
) -> Result<PathBuf> {
    let path = model_path(cache_dir, spec);
    if path.is_file() {
        return Ok(path);
    }
    std::fs::create_dir_all(cache_dir)?;
    let resp = reqwest::get(spec.url).await?.error_for_status()?;
    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
        on_progress(bytes.len() as u64, total);
    }
    verify(spec, &bytes)?;
    let tmp = path.with_extension("onnx.part");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}
