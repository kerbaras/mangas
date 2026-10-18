//! A window saving a setting after another process (the CLI) moved the data folder.

use std::fs;
use std::path::Path;

use mangas_core::library::{self, edit_settings, get_settings, Settings};
use mangas_core::storage::{move_data_dir, Folders};

fn folders(root: &Path) -> Folders {
    Folders {
        data_dir: root.join("data"),
        database: root.join("app/mangas.db"),
        covers: root.join("app/covers"),
        logs: root.join("app/logs"),
    }
}

#[test]
fn a_save_after_a_cli_storage_move_keeps_the_new_data_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    fs::create_dir_all(f.database.parent().unwrap()).unwrap();
    fs::create_dir_all(f.data_dir.join("chapters")).unwrap();
    let app = library::open(&f.database).unwrap();
    let cli = library::open(&f.database).unwrap();
    let loaded = get_settings(&app).unwrap();

    let moved =
        move_data_dir(&cli, &f.data_dir, &tmp.path().join("big-disk"), &mut |_| {}).unwrap();
    let edited = Settings {
        default_device: Some("kindle-scribe".into()),
        ..loaded.clone()
    };
    let saved = edit_settings(&app, Some(&loaded), &edited).unwrap();

    assert_eq!(saved.data_dir.as_deref(), Some(moved.data_dir.as_str()));
    assert_eq!(saved.default_device.as_deref(), Some("kindle-scribe"));
    assert_eq!(get_settings(&cli).unwrap(), saved);
}
