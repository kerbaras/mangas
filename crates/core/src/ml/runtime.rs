//! ONNX Runtime session setup: execution provider choice per platform, CPU fallback always.

use std::path::Path;

use ort::ep::ExecutionProviderDispatch;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;

use crate::domain::{Error, Result};

pub fn ort_err<T>(e: ort::Error<T>) -> Error {
    Error::Other(format!("onnx: {e}"))
}

/// Name of the accelerator this build prefers (the CPU provider is always appended).
pub fn provider_name() -> &'static str {
    if cfg!(feature = "cuda") {
        "cuda"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "coreml"
    } else if cfg!(windows) {
        "directml"
    } else {
        "cpu"
    }
}

fn providers() -> Vec<ExecutionProviderDispatch> {
    let mut eps: Vec<ExecutionProviderDispatch> = Vec::new();
    #[cfg(feature = "cuda")]
    eps.push(ort::ep::CUDA::default().build());
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    eps.push(ort::ep::CoreML::default().build());
    #[cfg(windows)]
    eps.push(ort::ep::DirectML::default().build());
    eps.push(ort::ep::CPU::default().build());
    eps
}

pub fn load_session(path: &Path) -> Result<Session> {
    Session::builder()
        .map_err(ort_err)?
        .with_execution_providers(providers())
        .map_err(ort_err)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort_err)?
        .commit_from_file(path)
        .map_err(ort_err)
}
