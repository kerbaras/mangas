//! The ONNX Runtime library, downloaded on demand into `<models>/runtime/<version>/` (so it moves
//! with the data folder and is cleared with the models, like them) and loaded by `ort`'s
//! `load-dynamic` mode. Release builds therefore ship without it.

mod extract;
mod install;
#[cfg(feature = "ml")]
mod load;
mod specs;
mod state;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::Error;

pub use extract::extract_member;
pub use install::{check, ensure};
#[cfg(feature = "ml")]
pub use load::ensure_loaded;
pub use specs::RUNTIMES;
pub use state::clear as forget_failure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archive {
    Tgz,
    Zip,
}

/// One official runtime build: the archive to fetch and the library to keep from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeSpec {
    /// `std::env::consts::OS` / `ARCH` of the target it runs on.
    pub os: &'static str,
    pub arch: &'static str,
    pub version: &'static str,
    pub url: &'static str,
    pub archive: Archive,
    /// Hex SHA-256 of the archive at `url`.
    pub sha256: &'static str,
    /// Archive size in bytes.
    pub size: u64,
    /// Path of the shared library inside the archive.
    pub member: &'static str,
    /// File name the library is installed under.
    pub lib_name: &'static str,
    /// Hex SHA-256 of the library, checked again before every load.
    pub lib_sha256: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RuntimeStatus {
    /// Built without the `ml` feature, or no official runtime exists for this platform.
    Unavailable,
    NotDownloaded {
        version: String,
    },
    Downloaded {
        version: String,
    },
    Failed {
        version: String,
        reason: String,
    },
}

pub fn for_target(os: &str, arch: &str) -> Option<RuntimeSpec> {
    RUNTIMES
        .iter()
        .find(|r| r.os == os && r.arch == arch)
        .copied()
}

/// The runtime for the platform this binary runs on.
pub fn current() -> Option<RuntimeSpec> {
    for_target(std::env::consts::OS, std::env::consts::ARCH)
}

pub fn root(models_dir: &Path) -> PathBuf {
    models_dir.join("runtime")
}

pub fn lib_path(models_dir: &Path, spec: &RuntimeSpec) -> PathBuf {
    root(models_dir).join(spec.version).join(spec.lib_name)
}

pub(crate) fn unsupported() -> Error {
    Error::Other(format!(
        "no ONNX Runtime build for {}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ))
}

pub fn status(models_dir: &Path) -> RuntimeStatus {
    match current().filter(|_| super::ENABLED) {
        None => RuntimeStatus::Unavailable,
        Some(spec) => status_of(models_dir, &spec, state::failure(models_dir)),
    }
}

/// Status of `spec` given the last recorded failure (pure apart from one `is_file`).
pub fn status_of(models_dir: &Path, spec: &RuntimeSpec, failure: Option<String>) -> RuntimeStatus {
    let version = spec.version.to_string();
    match (failure, lib_path(models_dir, spec).is_file()) {
        (Some(reason), _) => RuntimeStatus::Failed { version, reason },
        (None, true) => RuntimeStatus::Downloaded { version },
        (None, false) => RuntimeStatus::NotDownloaded { version },
    }
}
