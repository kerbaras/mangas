//! The models-folder lease seen from two processes: this test binary re-runs itself as the
//! holder (`child_holds_the_lease`), and the parent checks every path that changes the folder.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

use mangas_core::library::open_in_memory;
use mangas_core::ml::lease::{acquire, lease_path, Purpose, LEASE_FILE};
use mangas_core::storage::{clear_cache, move_data_dir, Cache, Folders};

const DIR_VAR: &str = "MANGAS_LEASE_TEST_DIR";

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
#[ignore = "run by a_download_in_another_process_blocks_move_remove_and_clear"]
fn child_holds_the_lease() {
    let Ok(dir) = std::env::var(DIR_VAR) else {
        return;
    };
    let dir = Path::new(&dir);
    let _lease = acquire(&dir.join("data/models"), Purpose::Download).unwrap();
    fs::write(dir.join("ready"), b"").unwrap();
    wait_for(&dir.join("done"));
}

#[test]
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
    assert!(
        !lease_path(&models).exists(),
        "released when the holder ends"
    );
    assert!(acquire(&models, Purpose::Remove).is_ok());
}

#[test]
fn a_move_refuses_a_download_and_a_download_refuses_a_move() {
    let dir = tempfile::tempdir().unwrap();
    let moving = acquire(dir.path(), Purpose::Move).unwrap();
    let err = acquire(dir.path(), Purpose::Download)
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("the data folder is moving"), "{err}");
    drop(moving);
    let downloading = acquire(dir.path(), Purpose::Download).unwrap();
    let err = acquire(dir.path(), Purpose::Move)
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("detector support is downloading"), "{err}");
    drop(downloading);
}

#[test]
fn a_lease_nobody_renews_is_taken_over() {
    let dir = tempfile::tempdir().unwrap();
    let path = lease_path(dir.path());
    fs::write(&path, br#"{"pid":999999,"purpose":"download"}"#).unwrap();
    let old = SystemTime::now() - Duration::from_secs(120);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(old)
        .unwrap();
    assert!(acquire(dir.path(), Purpose::Remove).is_ok());
}

#[test]
fn a_move_leaves_the_lease_behind() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    fs::create_dir_all(data.join("models")).unwrap();
    fs::write(data.join("models/model.onnx"), b"weights").unwrap();
    let lib = open_in_memory().unwrap();
    let to = root.path().join("moved");
    let moved = move_data_dir(&lib, &data, &to, &mut |_| {}).unwrap();
    assert_eq!(moved.files, 1);
    assert!(to.join("models/model.onnx").exists());
    assert!(!to.join("models").join(LEASE_FILE).exists());
    assert!(!data.join("models").exists());
}
