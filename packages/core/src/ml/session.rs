//! ONNX Runtime session setup: execution provider choice per platform, CPU fallback always.
//! Windows runs on CPU: the official runtime archive has no DirectML provider.

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
    } else {
        "cpu"
    }
}

/// Platform accelerators in preference order; ort skips any that is unavailable at runtime.
fn accelerators() -> Vec<ExecutionProviderDispatch> {
    [cuda(), coreml()].into_iter().flatten().collect()
}

#[cfg(feature = "cuda")]
fn cuda() -> Option<ExecutionProviderDispatch> {
    Some(ort::ep::CUDA::default().build())
}
#[cfg(not(feature = "cuda"))]
fn cuda() -> Option<ExecutionProviderDispatch> {
    None
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn coreml() -> Option<ExecutionProviderDispatch> {
    Some(ort::ep::CoreML::default().build())
}
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn coreml() -> Option<ExecutionProviderDispatch> {
    None
}

fn providers() -> Vec<ExecutionProviderDispatch> {
    accelerators()
        .into_iter()
        .chain(std::iter::once(ort::ep::CPU::default().build()))
        .collect()
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
