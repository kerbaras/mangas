//! Loads the downloaded library into `ort` once, before the first session.

use std::path::Path;
use std::sync::Mutex;

use super::install::check;
use super::{current, lib_path, state};
use crate::domain::IoAt;
use crate::domain::{Error, Result};

/// What this process did with the runtime. `ort` keeps the first library it loads for the life of
/// the process, and once its own load failed it can never load another one.
enum Loaded {
    No,
    Yes,
    Broken(String),
}

static LOADED: Mutex<Loaded> = Mutex::new(Loaded::No);

/// Loads the runtime for `ort`; every failure is recorded in `state` for the status to show.
pub fn ensure_loaded(models_dir: &Path) -> Result<()> {
    let mut loaded = LOADED
        .lock()
        .map_err(|_| Error::Other("onnx runtime: lock poisoned".into()))?;
    let result = match &*loaded {
        Loaded::Yes => return Ok(()),
        Loaded::Broken(reason) => Err(Error::Other(reason.clone())),
        Loaded::No => load(models_dir, &mut loaded),
    };
    match &result {
        Ok(()) => state::clear(),
        // Removed meanwhile: the runtime is absent, not failed.
        Err(_) if !installed(models_dir) => {}
        Err(e) => state::record(models_dir, e.to_string()),
    }
    result
}

fn installed(models_dir: &Path) -> bool {
    current().is_some_and(|spec| lib_path(models_dir, &spec).is_file())
}

/// Whether `ort` has a runtime: no session may be built before this is true.
pub fn is_loaded() -> bool {
    LOADED.lock().is_ok_and(|l| matches!(*l, Loaded::Yes))
}

/// Whether this process has tried to load the runtime (loaded or broken): from then on its library
/// is mapped or its `ort` load is final, so checking again here costs nothing new.
pub fn attempted() -> bool {
    LOADED.lock().is_ok_and(|l| !matches!(*l, Loaded::No))
}

fn load(models_dir: &Path, loaded: &mut Loaded) -> Result<()> {
    let spec = current().ok_or_else(super::unsupported)?;
    let path = lib_path(models_dir, &spec);
    if !path.is_file() {
        return Err(Error::Other("ONNX Runtime is not downloaded".into()));
    }
    check(
        &std::fs::read(&path).at(&path)?,
        spec.lib_sha256,
        "ONNX Runtime library",
    )?;
    // Opened here first: `ort` only says "dlopen failed", and a failed `ort` load is final.
    let _probe = open(&path)?;
    match ort::init_from(&path) {
        Ok(builder) => {
            let _first = builder.commit();
            *loaded = Loaded::Yes;
            Ok(())
        }
        Err(e) => {
            let reason = format!("onnx runtime: {e}; restart mangas to try again");
            *loaded = Loaded::Broken(reason.clone());
            Err(Error::Other(reason))
        }
    }
}

/// Whether the library at `path` opens and exports the `ort` entry point, with the system
/// loader's own message when it doesn't. Leaves `ort` untouched.
pub fn probe(path: &Path) -> Result<()> {
    open(path).map(drop)
}

/// Opens the library the way `ort` does, keeping the system loader's own message.
fn open(path: &Path) -> Result<libloading::Library> {
    let failed = |e: &dyn std::error::Error| {
        Error::Other(format!(
            "onnx runtime {}: {}",
            path.display(),
            error_chain(e)
        ))
    };
    // SAFETY: the file is the pinned official build, its SHA-256 was just checked.
    let lib = unsafe { libloading::Library::new(path) }.map_err(|e| failed(&e))?;
    // SAFETY: only the symbol's presence is checked; it is never called.
    unsafe { lib.get::<unsafe extern "C" fn()>(b"OrtGetApiBase") }.map_err(|e| failed(&e))?;
    Ok(lib)
}

/// `e` and its sources: "dlopen failed: dlopen(…): tried: … (no such file)".
pub fn error_chain(e: &dyn std::error::Error) -> String {
    let mut text = e.to_string();
    let mut source = e.source();
    while let Some(s) = source {
        let part = s.to_string();
        if !text.contains(&part) {
            text = format!("{text}: {part}");
        }
        source = s.source();
    }
    text
}
