//! `write_cbz` only names the CBZ by renaming its finished `.part` (#301).

use std::path::Path;

use mangas_core::download::write_cbz;

fn pages(dir: &Path) {
    for name in ["0001.png", "0002.jpg"] {
        std::fs::write(dir.join(name), name).unwrap();
    }
}

fn entries(cbz: &Path) -> usize {
    zip::ZipArchive::new(std::fs::File::open(cbz).unwrap())
        .unwrap()
        .len()
}

#[test]
fn a_part_left_by_a_killed_run_is_taken_over() {
    let dir = tempfile::tempdir().unwrap();
    pages(dir.path());
    let out = dir.path().join("1.cbz");
    std::fs::write(dir.path().join("1.cbz.part"), "half a zip").unwrap();
    assert_eq!(write_cbz(dir.path(), &out).unwrap(), 2);
    assert_eq!(entries(&out), 2);
    assert!(!dir.path().join("1.cbz.part").exists());
}

#[test]
fn a_new_pack_replaces_the_old_cbz() {
    let dir = tempfile::tempdir().unwrap();
    pages(dir.path());
    let out = dir.path().join("1.cbz");
    std::fs::write(&out, "old").unwrap();
    write_cbz(dir.path(), &out).unwrap();
    assert_eq!(entries(&out), 2);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[cfg(unix)]
#[test]
fn a_failed_pack_keeps_the_old_cbz_and_leaves_no_part() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    pages(dir.path());
    let unreadable = dir.path().join("0002.jpg");
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&unreadable).is_ok() {
        return; // root reads it anyway
    }
    let out = dir.path().join("1.cbz");
    std::fs::write(&out, "old").unwrap();
    assert!(write_cbz(dir.path(), &out).is_err());
    assert_eq!(std::fs::read(&out).unwrap(), b"old");
    assert!(!dir.path().join("1.cbz.part").exists());
}
