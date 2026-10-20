//! The export lease (#172): exports share it, a data-folder move takes it exclusively, and it is
//! separate from the detector lease an export may need exclusively to download detector support.

use std::cell::Cell;
use std::path::PathBuf;

use mangas_core::library::open_in_memory;
use mangas_core::ml::lease::{acquire, acquire_at, Purpose};
use mangas_core::storage::{hold_for_export, move_data_dir, EXPORT_LEASE};
use mangas_core::Error;

fn at(dir: PathBuf) -> impl Fn() -> Result<PathBuf, Error> {
    move || Ok(dir.clone())
}

fn moving(dir: &std::path::Path) -> String {
    match acquire_at(dir, EXPORT_LEASE, Purpose::Move) {
        Ok(_) => "free".into(),
        Err(e) => e.to_string(),
    }
}

#[test]
fn exports_share_the_lease_and_a_move_excludes_them_both_ways() {
    let dir = tempfile::tempdir().unwrap();
    let first = hold_for_export(at(dir.path().into())).unwrap();
    let second = hold_for_export(at(dir.path().into())).unwrap();
    assert!(moving(dir.path()).contains("an export or import is running"));
    drop((first, second));
    let held = acquire_at(dir.path(), EXPORT_LEASE, Purpose::Move).unwrap();
    let err = hold_for_export(at(dir.path().into()))
        .err()
        .unwrap()
        .to_string();
    assert!(
        err.contains("the data folder is moving (Mangas process"),
        "{err}"
    );
    drop(held);
}

#[test]
fn an_export_can_still_download_detector_support() {
    let dir = tempfile::tempdir().unwrap();
    let _reading = hold_for_export(at(dir.path().into())).unwrap();
    acquire(&dir.path().join("models"), Purpose::Download).unwrap();
}

#[test]
fn a_move_that_switched_the_folder_before_the_lock_gets_the_new_folder_locked() {
    let (old, new) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let calls = Cell::new(0);
    let data_dir = || {
        calls.set(calls.get() + 1);
        let dir = if calls.get() == 1 {
            old.path()
        } else {
            new.path()
        };
        Ok::<_, Error>(dir.to_path_buf())
    };
    let _reading = hold_for_export(data_dir).unwrap();
    assert!(moving(new.path()).contains("an export or import is running"));
    assert_eq!(moving(old.path()), "free");
}

#[test]
fn a_move_an_export_refuses_never_takes_the_detector_lease() {
    let (data, to) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let lib = open_in_memory().unwrap();
    let _reading = hold_for_export(at(data.path().into())).unwrap();
    // The export is downloading its detectors: a move must be refused by the export lease, before
    // it ever reaches the detector lease.
    let _detectors = acquire(&data.path().join("models"), Purpose::Download).unwrap();
    let err = move_data_dir(&lib, data.path(), &to.path().join("new"), &mut |_| {})
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("an export or import is running"), "{err}");
}
