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

/// ML Program with static shapes: CoreML's default NeuralNetwork format builds a session for the
/// RT-DETR bubble model and then fails every run ("Unable to compute the prediction"), and ML
/// Program without static shapes doesn't compile it (#200).
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn coreml() -> Option<ExecutionProviderDispatch> {
    use ort::ep::coreml::ModelFormat;
    let ep = ort::ep::CoreML::default()
        .with_model_format(ModelFormat::MLProgram)
        .with_static_input_shapes(true);
    Some(ep.build())
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

/// A session on the platform accelerator, or on the CPU alone when the accelerator can't build it
/// (e.g. an op CoreML lacks on an older macOS).
pub fn load_session(path: &Path) -> Result<Session> {
    let cpu = || vec![ort::ep::CPU::default().build()];
    if accelerators().is_empty() {
        return session(path, cpu());
    }
    session(path, providers()).or_else(|e| {
        tracing::warn!(
            "onnx: {} can't run {}, using the CPU: {e}",
            provider_name(),
            path.display()
        );
        session(path, cpu())
    })
}

fn session(path: &Path, providers: Vec<ExecutionProviderDispatch>) -> Result<Session> {
    Session::builder()
        .map_err(ort_err)?
        .with_execution_providers(providers)
        .map_err(ort_err)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort_err)?
        .commit_from_file(path)
        .map_err(ort_err)
}
