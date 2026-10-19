//! #211: deleting detector support from Storage forgets the runtime's last load failure, so the
//! status says "not downloaded" instead of the stale loader error.
#![cfg(feature = "ml")]

use mangas_core::ml::{self, runtime, RuntimeStatus};
use mangas_core::storage::{clear_cache, Cache, Folders};

#[test]
fn clearing_the_models_cache_forgets_the_runtime_failure() {
    let root = tempfile::tempdir().unwrap();
    let folders = Folders {
        data_dir: root.path().join("data"),
        database: root.path().join("app/mangas.db"),
        covers: root.path().join("app/covers"),
        logs: root.path().join("app/logs"),
    };
    let models = folders.models();
    let lib = runtime::lib_path(&models, &runtime::current().unwrap());
    std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
    std::fs::write(&lib, b"not a library").unwrap();
    assert!(runtime::ensure_loaded(&models).is_err());
    let before = ml::status(&models).runtime;
    assert!(matches!(before, RuntimeStatus::Failed { .. }), "{before:?}");

    clear_cache(&folders, Cache::Models).unwrap();

    let after = ml::status(&models).runtime;
    assert!(
        matches!(after, RuntimeStatus::NotDownloaded { .. }),
        "{after:?}"
    );
}
