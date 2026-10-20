//! The models-folder lease seen from two processes: this test binary re-runs itself as the
//! holder (`child_holds_the_lease`), and the parent checks every path that changes the folder.
//! One test runs both cases in turn: a fork from a parallel test would briefly inherit the lock.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use mangas_core::library::open_in_memory;
use mangas_core::ml::lease::{acquire, lease_path, Purpose};
use mangas_core::storage::{clear_cache, move_data_dir, Cache, Folders};

const DIR_VAR: &str = "MANGAS_LEASE_TEST_DIR";
const CRASH_VAR: &str = "MANGAS_LEASE_TEST_CRASH";

fn wait_for(path: &Path) {
    let start = Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "{} never appeared",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "run by two_processes_on_one_models_folder"]
fn child_holds_the_lease() {
    let Ok(dir) = std::env::var(DIR_VAR) else {
        return;
    };
    let dir = Path::new(&dir);
    let _lease = acquire(&dir.join("data/models"), Purpose::Download).unwrap();
    fs::write(dir.join("ready"), b"").unwrap();
    if std::env::var(CRASH_VAR).is_ok() {
        std::process::exit(0); // ends without dropping the lease, like a crash
    }
    wait_for(&dir.join("done"));
}

#[test]
fn two_processes_on_one_models_folder() {
    a_download_in_another_process_blocks_move_remove_and_clear();
    a_holder_that_dies_frees_the_folder();
}

fn a_download_in_another_process_blocks_move_remove_and_clear() {
    let root = tempfile::tempdir().unwrap();
    let models = root.path().join("data/models");
    fs::create_dir_all(&models).unwrap();
    fs::write(models.join("model.onnx"), b"weights").unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "child_holds_the_lease",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(DIR_VAR, root.path())
        .spawn()
        .unwrap();
    wait_for(&root.path().join("ready"));

    let refused = |e: String| assert!(e.contains("detector support is downloading"), "{e}");
    refused(mangas_core::ml::remove(&models).unwrap_err().to_string());
    let folders = Folders {
        data_dir: root.path().join("data"),
        database: root.path().join("app/mangas.db"),
        covers: root.path().join("app/covers"),
        logs: root.path().join("app/logs"),
    };
    refused(
        clear_cache(&folders, Cache::Models)
            .unwrap_err()
            .to_string(),
    );
    let lib = open_in_memory().unwrap();
    let to = root.path().join("moved");
    let moved = move_data_dir(&lib, &root.path().join("data"), &to, &mut |_| {});
    refused(moved.unwrap_err().to_string());
    refused(
        acquire(&models, Purpose::Download)
            .err()
            .unwrap()
            .to_string(),
    );
    assert!(models.join("model.onnx").exists(), "nothing was touched");

    fs::write(root.path().join("done"), b"").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(acquire(&models, Purpose::Remove).is_ok());
}

fn a_holder_that_dies_frees_the_folder() {
    let root = tempfile::tempdir().unwrap();
    let models = root.path().join("data/models");
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "child_holds_the_lease",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(DIR_VAR, root.path())
        .env(CRASH_VAR, "1")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        lease_path(&models).exists(),
        "the lock file outlives the holder"
    );
    let reading = mangas_core::ml::lease::share(&models).unwrap();
    let err = acquire(&models, Purpose::Remove).err().unwrap().to_string();
    assert!(
        err.contains("an export or import is running"),
        "the dead downloader isn't blamed: {err}"
    );
    drop(reading);
    assert!(
        acquire(&models, Purpose::Download).is_ok(),
        "the lock does not outlive it"
    );
}
