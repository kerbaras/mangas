//! `mangas ml` against a temporary home (no network: download is not exercised here).

use std::path::Path;
use std::process::{Command, Output};

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn status_matches_the_build() {
    let home = tempfile::tempdir().unwrap();
    let status = stdout(&mangas(home.path(), &["ml"]));
    if cfg!(feature = "ml") {
        assert!(status.contains("onnxruntime"), "{status}");
        assert!(status.contains("not downloaded"), "{status}");
        assert!(status.contains("Run `mangas ml download`"), "{status}");
    } else {
        assert!(status.contains("no detector support"), "{status}");
        let out = mangas(home.path(), &["ml", "download"]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("`ml` feature"));
    }
}

#[test]
fn remove_deletes_runtime_and_models() {
    let home = tempfile::tempdir().unwrap();
    let models = home.path().join("data/mangas/models");
    std::fs::create_dir_all(models.join("runtime/1.28.3")).unwrap();
    std::fs::write(models.join("runtime/1.28.3/libonnxruntime.so"), [0u8; 1000]).unwrap();
    std::fs::write(models.join("anime-face-detect-v0-n.onnx"), [0u8; 500]).unwrap();
    let removed = stdout(&mangas(home.path(), &["ml", "remove"]));
    assert!(
        removed.contains("Removed detector support: 1.50 KB freed"),
        "{removed}"
    );
    assert!(!models.join("runtime").exists());
    assert!(!models.join("anime-face-detect-v0-n.onnx").exists());
}
