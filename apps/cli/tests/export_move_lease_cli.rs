//! An export and a data-folder move exclude each other across processes (#172): the export holds
//! the export lease shared while it reads chapters, the move takes it exclusively.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::ml::lease::{acquire_at, Purpose};
use mangas_core::storage::{hold_for_export, EXPORT_LEASE};

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn data(home: &Path) -> std::path::PathBuf {
    home.join("data/mangas")
}

#[test]
fn a_move_is_refused_while_an_export_reads_chapters() {
    let home = tempfile::tempdir().unwrap();
    let reading = hold_for_export(|| Ok::<_, mangas_core::Error>(data(home.path()))).unwrap();
    let to = home.path().join("elsewhere");
    let out = mangas(home.path(), &["storage", "move", to.to_str().unwrap()]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("an export is running; try again when it finishes"),
        "{err}"
    );
    drop(reading);
}

#[test]
fn an_export_is_refused_while_the_data_folder_moves_before_it_reads_a_chapter() {
    let home = tempfile::tempdir().unwrap();
    let moving = acquire_at(&data(home.path()), EXPORT_LEASE, Purpose::Move).unwrap();
    let out = mangas(home.path(), &["export", "any-series", "-o", "/nonexistent"]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    let kindle = mangas(home.path(), &["kindle", "/nonexistent/chapter"]);
    let err = String::from_utf8_lossy(&kindle.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    drop(moving);
}
