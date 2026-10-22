//! Download → verify → extract → atomic write of the ONNX Runtime library.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use super::{lib_path, root, state, RuntimeSpec};
use crate::domain::IoAt;
use crate::domain::{Error, Result};
use crate::library::Network;
use crate::ml::cache::sha256_hex;
use crate::ml::fetch::{fetch, is_cancelled};

/// Errors unless `bytes` hash to `want`.
pub fn check(bytes: &[u8], want: &str, what: &str) -> Result<()> {
    if sha256_hex(bytes) == want {
        Ok(())
    } else {
        Err(Error::Other(format!(
            "{what}: sha256 mismatch (expected {want})"
        )))
    }
}

/// Returns the library path for `spec`, downloading and installing it first if it is missing or
/// damaged. Failures other than a cancel are kept for [`super::status`].
pub async fn ensure(
    models_dir: &Path,
    spec: &RuntimeSpec,
    network: &Network,
    on_progress: &mut (dyn FnMut(u64, Option<u64>) + Send),
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let result = install(models_dir, spec, network, on_progress, cancel).await;
    match &result {
        Ok(_) => state::clear(),
        Err(e) if !is_cancelled(e) => state::record(models_dir, e.to_string()),
        Err(_) => {}
    }
    result
}

async fn install(
    models_dir: &Path,
    spec: &RuntimeSpec,
    network: &Network,
    on_progress: &mut (dyn FnMut(u64, Option<u64>) + Send),
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let path = lib_path(models_dir, spec);
    if path.is_file() && check(&std::fs::read(&path).at(&path)?, spec.lib_sha256, "").is_ok() {
        return Ok(path);
    }
    let archive = fetch(spec.url, network, on_progress, cancel).await?;
    check(&archive, spec.sha256, "ONNX Runtime archive")?;
    let lib = super::extract_member(&archive, spec.archive, spec.member)?;
    check(&lib, spec.lib_sha256, "ONNX Runtime library")?;
    let dir = root(models_dir).join(spec.version);
    std::fs::create_dir_all(&dir).at(&dir)?;
    let tmp = dir.join(format!("{}.part", spec.lib_name));
    std::fs::write(&tmp, &lib).at(&tmp)?;
    std::fs::rename(&tmp, &path).at(&path)?;
    prune_other_versions(&root(models_dir), spec.version);
    Ok(path)
}

/// Best effort: a runtime from an older app version is no longer used.
fn prune_other_versions(root: &Path, keep: &str) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name() != keep {
            if let Err(e) = std::fs::remove_dir_all(entry.path()) {
                log::warn!("onnx runtime: can't remove {}: {e}", entry.path().display());
            }
        }
    }
}
