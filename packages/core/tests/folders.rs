//! The file-manager guard: only real folders inside the app's folders or an export folder.

use std::fs;
use std::path::{Path, PathBuf};

use mangas_core::library::{export_folders, open_in_memory, openable_folder, record_export};

fn tree() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join(".local/share/mangas");
    let outside = tmp.path().join("elsewhere");
    fs::create_dir_all(data.join("chapters/95/80")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(data.join("chapters/95/80/0001.jpg"), b"x").unwrap();
    fs::write(outside.join("run.sh"), b"#!/bin/sh").unwrap();
    (tmp, data, outside)
}

#[test]
fn opens_folders_inside_a_root_even_under_dot_folders() {
    let (_tmp, data, _) = tree();
    let dir = data.join("chapters/95/80");
    assert_eq!(
        openable_folder(&dir, std::slice::from_ref(&data)).unwrap(),
        dir.canonicalize().unwrap()
    );
    assert!(openable_folder(&data, std::slice::from_ref(&data)).is_ok());
}

#[test]
fn refuses_files_missing_paths_and_folders_outside_the_roots() {
    let (_tmp, data, outside) = tree();
    let roots = [data.clone()];
    assert!(openable_folder(&data.join("chapters/95/80/0001.jpg"), &roots).is_err());
    assert!(openable_folder(&outside.join("run.sh"), &roots).is_err());
    assert!(openable_folder(&outside, &roots).is_err());
    assert!(openable_folder(&data.join("missing"), &roots).is_err());
    assert!(openable_folder(&data.join("../../../elsewhere"), &roots).is_err());
    assert!(openable_folder(&data, &[]).is_err());
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_a_root_is_refused() {
    let (_tmp, data, outside) = tree();
    let link = data.join("escape");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    assert!(openable_folder(&link, &[data]).is_err());
}

#[test]
fn export_folders_are_the_folders_of_recorded_exports() {
    let lib = open_in_memory().unwrap();
    let file = |p: &str| Path::new(p).to_path_buf();
    record_export(&lib, &file("/home/me/Kindle/a.epub"), "A", "kpw").unwrap();
    record_export(&lib, &file("/home/me/Kindle/b.epub"), "B", "kpw").unwrap();
    record_export(&lib, &file("/media/usb/c.epub"), "C", "kpw").unwrap();
    assert_eq!(
        export_folders(&lib).unwrap(),
        [file("/media/usb"), file("/home/me/Kindle")]
    );
}
