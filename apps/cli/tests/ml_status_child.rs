//! `ml status --json` run as a child process is the desktop's launch status check (#210): the app
//! never maps the ONNX Runtime for a status, so Windows can delete it without a restart.

use std::path::PathBuf;
use std::process::Command;

use mangas_core::ml::{self, child, RuntimeStatus, Status};

/// The CLI under test; `MANGAS_EXE` points it at the shipped `mangas` binary instead.
fn exe() -> PathBuf {
    std::env::var_os("MANGAS_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_mangas-cli").into())
}

#[test]
fn the_json_status_is_the_shared_status_type() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(exe())
        .args(child::args(dir.path()))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let status: Status = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(status, ml::status(dir.path()));
}

#[cfg(feature = "ml")]
#[test]
fn a_runtime_that_wont_load_fails_in_the_child_not_here() {
    let dir = tempfile::tempdir().unwrap();
    let spec = ml::runtime::current().unwrap();
    let lib = ml::runtime::lib_path(dir.path(), &spec);
    std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
    std::fs::write(&lib, b"not a library").unwrap();
    for m in &ml::MODELS {
        std::fs::write(ml::cache::model_path(dir.path(), m), b"not a model").unwrap();
    }
    let status = child::probed_status(dir.path(), &exe());
    assert!(!status.downloaded);
    match status.runtime {
        RuntimeStatus::Failed { reason, .. } => assert!(reason.contains("sha256"), "{reason}"),
        other => panic!("{other:?}"),
    }
    assert!(
        !ml::runtime::attempted(),
        "this process never touched the runtime"
    );
}

/// The runtime and models on disk, fetched without loading the runtime here (`ml::download`
/// loads it, as a detector run would).
#[cfg(feature = "ml")]
fn downloaded() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let network = mangas_core::library::Settings::default().network();
    let never = std::sync::atomic::AtomicBool::new(false);
    let spec = ml::runtime::current().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        ml::runtime::ensure(dir.path(), &spec, &network, &mut |_, _| {}, &never).await?;
        for model in &ml::MODELS {
            ml::cache::ensure_model(dir.path(), model, &network, &mut |_, _| {}, &never).await?;
        }
        Ok::<_, mangas_core::domain::Error>(())
    })
    .unwrap();
    assert!(ml::status(dir.path()).downloaded);
    assert!(!ml::runtime::attempted());
    dir
}

/// Status, then Remove in the same process, with no restart: the runtime file is gone.
#[cfg(feature = "ml")]
#[test]
#[ignore = "downloads ~190 MB; run alone (`--ignored --exact`), as the Windows CI job does"]
fn remove_after_the_launch_status_check_needs_no_restart() {
    let dir = downloaded();
    let lib = ml::runtime::lib_path(dir.path(), &ml::runtime::current().unwrap());
    let status = child::probed_status(dir.path(), &exe());
    assert!(
        matches!(status.runtime, RuntimeStatus::Downloaded { .. }),
        "{status:?}"
    );
    assert!(!ml::runtime::attempted());
    ml::remove(dir.path()).unwrap();
    assert!(!lib.exists(), "{} still there", lib.display());
    assert!(!ml::runtime::root(dir.path()).exists());
    assert!(!ml::runtime::pending_marker(dir.path()).exists());
    let after = ml::status(dir.path()).runtime;
    assert!(
        matches!(after, RuntimeStatus::NotDownloaded { .. }),
        "{after:?}"
    );
}

/// The old in-process check, for contrast: Windows keeps the loaded DLL until the process exits.
#[cfg(feature = "ml")]
#[test]
#[ignore = "downloads ~190 MB; run alone (`--ignored --exact`), as the Windows CI job does"]
fn an_in_process_check_keeps_the_runtime_mapped() {
    let dir = downloaded();
    let lib = ml::runtime::lib_path(dir.path(), &ml::runtime::current().unwrap());
    let status = ml::checked_status(dir.path());
    assert!(ml::runtime::is_loaded(), "{status:?}");
    ml::remove(dir.path()).unwrap();
    let after = ml::status(dir.path()).runtime;
    if cfg!(windows) {
        assert!(lib.exists(), "Windows deleted a loaded DLL?");
        assert!(
            matches!(after, RuntimeStatus::RemovePending { .. }),
            "{after:?}"
        );
    } else {
        assert!(!lib.exists(), "{after:?}");
    }
}
