//! Loads the downloaded library into `ort` once, before the first session.

use std::path::Path;
use std::sync::Mutex;

use super::install::check;
use super::{current, lib_path, state};
use crate::domain::{Error, Result};

/// `ort` keeps the first library it loads for the life of the process.
static LOADED: Mutex<bool> = Mutex::new(false);

pub fn ensure_loaded(models_dir: &Path) -> Result<()> {
    let mut loaded = LOADED
        .lock()
        .map_err(|_| Error::Other("onnx runtime: lock poisoned".into()))?;
    if *loaded {
        return Ok(());
    }
    let result = load(models_dir);
    match &result {
        Ok(()) => {
            *loaded = true;
            state::clear();
        }
        Err(e) => state::record(e.to_string()),
    }
    result
}

fn load(models_dir: &Path) -> Result<()> {
    let spec = current().ok_or_else(super::unsupported)?;
    let path = lib_path(models_dir, &spec);
    if !path.is_file() {
        return Err(Error::Other("ONNX Runtime is not downloaded".into()));
    }
    check(
        &std::fs::read(&path)?,
        spec.lib_sha256,
        "ONNX Runtime library",
    )?;
    let builder = ort::init_from(&path).map_err(|e| Error::Other(format!("onnx runtime: {e}")))?;
    let _first = builder.commit();
    Ok(())
}
