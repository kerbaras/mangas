use std::path::{Path, PathBuf};

use mangas_core::send::usb::{choose, copy_to_kindle, find_kindles_in, looks_like_kindle};

fn mkdirs(root: &Path, dirs: &[&str]) {
    for d in dirs {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
}

#[test]
fn recognises_a_kindle_by_name_or_system_folder() {
    assert!(looks_like_kindle(
        Path::new("/media/me/Kindle"),
        true,
        false
    ));
    assert!(looks_like_kindle(Path::new("/Volumes/USB"), true, true));
    assert!(looks_like_kindle(
        Path::new("/run/user/1000/gvfs/mtp:host=Amazon_Kindle_G000/Internal Storage"),
        true,
        false
    ));
    assert!(!looks_like_kindle(
        Path::new("/media/me/Kindle"),
        false,
        true
    ));
    assert!(!looks_like_kindle(Path::new("/media/me/USB"), true, false));
}

#[test]
fn finds_kindles_under_fake_mount_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let media = tmp.path().join("media");
    mkdirs(
        &media,
        &[
            "me/Kindle/documents",
            "me/SCRIBE/Documents",
            "me/SCRIBE/system",
            "me/Photos/documents",
            "me/Empty",
        ],
    );
    let mtp = tmp.path().join("gvfs");
    mkdirs(
        &mtp,
        &["mtp:host=Amazon_Kindle_X/Internal Storage/documents"],
    );
    let roots: Vec<(PathBuf, usize)> = vec![(media.clone(), 2), (mtp, 2)];
    let found = find_kindles_in(&roots);
    let names: Vec<&str> = found.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["Amazon Kindle X", "Kindle", "SCRIBE"]);
    let scribe = &found[2];
    assert_eq!(scribe.documents, media.join("me/SCRIBE/Documents"));
}

#[test]
fn no_mount_roots_means_no_device() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(find_kindles_in(&[(tmp.path().join("missing"), 2)]).is_empty());
}

#[test]
fn copies_into_documents_with_progress() {
    let tmp = tempfile::tempdir().unwrap();
    mkdirs(tmp.path(), &["Kindle/documents"]);
    let kindle = find_kindles_in(&[(tmp.path().into(), 1)]).remove(0);
    let book = tmp.path().join("Vol 1.epub");
    let data = vec![7u8; 3 * 1024 * 1024 + 5];
    std::fs::write(&book, &data).unwrap();
    let mut seen = Vec::new();
    let copied =
        copy_to_kindle(&book, &kindle, &mut |done, total| seen.push((done, total))).unwrap();
    assert!(!copied.already_there);
    let dest = copied.path;
    assert_eq!(dest, kindle.documents.join("Vol 1.epub"));
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    let total = data.len() as u64;
    assert_eq!(seen.first(), Some(&(0, total)));
    assert_eq!(seen.last(), Some(&(total, total)));
    assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0));
}

#[test]
fn unplugged_device_and_missing_file_leave_nothing_behind() {
    let tmp = tempfile::tempdir().unwrap();
    mkdirs(tmp.path(), &["Kindle/documents"]);
    let kindle = find_kindles_in(&[(tmp.path().into(), 1)]).remove(0);
    let err = copy_to_kindle(&tmp.path().join("gone.epub"), &kindle, &mut |_, _| {});
    assert!(err.is_err());
    assert!(!kindle.documents.join("gone.epub").exists());
    let book = tmp.path().join("a.epub");
    std::fs::write(&book, b"x").unwrap();
    std::fs::remove_dir_all(tmp.path().join("Kindle")).unwrap();
    let err = copy_to_kindle(&book, &kindle, &mut |_, _| {}).unwrap_err();
    assert!(err.to_string().contains("no longer connected"), "{err}");
}

#[test]
fn choose_takes_an_explicit_mount_or_reports_device_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    mkdirs(tmp.path(), &["Reader/documents"]);
    let picked = choose(Some(&tmp.path().join("Reader")), Vec::new()).unwrap();
    assert_eq!(picked.name, "Reader");
    let err = choose(Some(tmp.path()), Vec::new())
        .unwrap_err()
        .to_string();
    assert!(err.starts_with("Device not found"), "{err}");
    let err = choose(None, Vec::new()).unwrap_err().to_string();
    assert!(
        err.starts_with("Device not found: connect your Kindle"),
        "{err}"
    );
}

#[test]
fn failed_replace_keeps_the_book_already_on_the_kindle() {
    let tmp = tempfile::tempdir().unwrap();
    mkdirs(tmp.path(), &["Kindle/documents"]);
    let kindle = find_kindles_in(&[(tmp.path().into(), 1)]).remove(0);
    let name = format!("{}.epub", "x".repeat(250));
    let onboard = kindle.documents.join(&name);
    std::fs::write(&onboard, b"old book").unwrap();
    let missing = tmp.path().join(&name);
    assert!(copy_to_kindle(&missing, &kindle, &mut |_, _| {}).is_err());
    assert_eq!(std::fs::read(&onboard).unwrap(), b"old book");
    let left: Vec<_> = std::fs::read_dir(&kindle.documents).unwrap().collect();
    assert_eq!(left.len(), 1, "only the old book remains: {left:?}");
    std::fs::write(&missing, b"new book").unwrap();
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| copy_to_kindle(&missing, &kindle, &mut |_, _| {}).unwrap());
        }
    });
    assert_eq!(std::fs::read(&onboard).unwrap(), b"new book");
    assert_eq!(std::fs::read_dir(&kindle.documents).unwrap().count(), 1);
}

#[test]
fn sending_a_book_already_in_documents_leaves_it_intact() {
    let tmp = tempfile::tempdir().unwrap();
    mkdirs(tmp.path(), &["Kindle/documents"]);
    let kindle = find_kindles_in(&[(tmp.path().into(), 1)]).remove(0);
    let onboard = kindle.documents.join("Vol 2.epub");
    std::fs::write(&onboard, b"book").unwrap();
    let mut seen = Vec::new();
    let copied = copy_to_kindle(&onboard, &kindle, &mut |d, t| seen.push((d, t))).unwrap();
    assert_eq!((copied.path, copied.already_there), (onboard.clone(), true));
    assert_eq!(std::fs::read(&onboard).unwrap(), b"book");
    assert_eq!(seen, vec![(4, 4)]);
}

#[cfg(unix)]
#[test]
fn read_only_kindle_says_so_once() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    mkdirs(tmp.path(), &["Kindle/documents"]);
    let kindle = find_kindles_in(&[(tmp.path().into(), 1)]).remove(0);
    let book = tmp.path().join("Vol 3.epub");
    std::fs::write(&book, b"book").unwrap();
    let lock = |mode| std::fs::set_permissions(&kindle.documents, PermissionsExt::from_mode(mode));
    lock(0o555).unwrap();
    if std::fs::write(kindle.documents.join("probe"), b"").is_ok() {
        return; // root ignores permissions
    }
    let err = copy_to_kindle(&book, &kindle, &mut |_, _| {})
        .unwrap_err()
        .to_string();
    lock(0o755).unwrap();
    let root = kindle.root.display();
    let want =
        format!("Kindle at {root} is read-only; eject and reconnect it, or check it isn't locked");
    assert_eq!(err, want);
}
