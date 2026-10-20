//! The models-folder lease within one process: exclusive changes, shared reads, and the lock
//! files surviving a clear and staying behind on a move.

use std::fs;

use mangas_core::library::open_in_memory;
use mangas_core::ml::lease::{acquire, lease_path, share, Purpose, LEASE_FILE};
use mangas_core::storage::{clear_cache, move_data_dir, Cache, Folders};

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
fn exports_share_the_folder_but_exclude_changes() {
    let dir = tempfile::tempdir().unwrap();
    let reading = share(dir.path()).unwrap();
    let second = share(dir.path()).unwrap();
    let err = acquire(dir.path(), Purpose::Remove)
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("an export or import is running"), "{err}");
    drop((reading, second));
    let downloading = acquire(dir.path(), Purpose::Download).unwrap();
    let err = share(dir.path()).err().unwrap().to_string();
    assert!(err.contains("detector support is downloading"), "{err}");
    drop(downloading);
    assert!(share(dir.path()).is_ok());
}

#[test]
fn clearing_the_models_keeps_the_lock_file() {
    let root = tempfile::tempdir().unwrap();
    let folders = Folders {
        data_dir: root.path().join("data"),
        database: root.path().join("app/mangas.db"),
        covers: root.path().join("app/covers"),
        logs: root.path().join("app/logs"),
    };
    let models = folders.models();
    fs::create_dir_all(&models).unwrap();
    fs::write(models.join("model.onnx"), b"weights").unwrap();
    let cleared = clear_cache(&folders, Cache::Models).unwrap();
    assert_eq!((cleared.files, cleared.bytes), (1, 7));
    assert!(lease_path(&models).exists());
    assert!(!models.join("model.onnx").exists());
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
