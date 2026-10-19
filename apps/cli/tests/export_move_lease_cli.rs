//! An export and a data-folder move exclude each other across processes (#172): the export holds
//! the data-folder lease shared while it reads chapters, the move takes it exclusively.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::ml::lease::{acquire, Purpose};
use mangas_core::storage::hold_for_export;

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn models(home: &Path) -> std::path::PathBuf {
    home.join("data/mangas/models")
}

#[test]
fn a_move_is_refused_while_an_export_reads_chapters() {
    let home = tempfile::tempdir().unwrap();
    let reading = hold_for_export(&models(home.path())).unwrap();
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
    let moving = acquire(&models(home.path()), Purpose::Move).unwrap();
    let out = mangas(home.path(), &["export", "any-series", "-o", "/nonexistent"]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    let kindle = mangas(home.path(), &["kindle", "/nonexistent/chapter"]);
    let err = String::from_utf8_lossy(&kindle.stderr);
    assert!(err.contains("the data folder is moving"), "{err}");
    drop(moving);
}
