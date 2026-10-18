//! A runtime the OS keeps locked (a loaded DLL on Windows) survives Remove and is reported.
#![cfg(unix)]

use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use mangas_core::ml::remove;
use mangas_core::ml::runtime::{for_target, lib_path, pending_marker, status_of, RuntimeStatus};

#[test]
fn a_locked_runtime_is_reported_until_a_later_remove_deletes_it() {
    let spec = for_target("linux", "x86_64").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let lib = lib_path(dir.path(), &spec);
    let folder = lib.parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(&lib, b"loaded runtime").unwrap();
    // A read-only folder stands in for the Windows lock.
    std::fs::set_permissions(&folder, Permissions::from_mode(0o555)).unwrap();
    remove(dir.path()).unwrap();
    let locked = lib.exists();
    let status = status_of(dir.path(), &spec, None);
    std::fs::set_permissions(&folder, Permissions::from_mode(0o755)).unwrap();
    if !locked {
        return; // root ignores the permission, nothing to observe
    }
    assert!(
        matches!(&status, RuntimeStatus::RemovePending { version, bytes } if version == spec.version && *bytes > 0),
        "{status:?}"
    );
    remove(dir.path()).unwrap();
    assert!(!pending_marker(dir.path()).exists());
    assert!(matches!(
        status_of(dir.path(), &spec, None),
        RuntimeStatus::NotDownloaded { .. }
    ));
}
