use std::path::Path;

use mangas_core::backup::{merge_settings, sanitize};
use mangas_core::library::{self, list_exports, record_export, record_sent, Settings};
use mangas_core::send::{EmailSettings, SendVia};

#[test]
fn records_exports_and_stamps_the_latest_delivery() {
    let lib = library::open_in_memory().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("vol-1.epub");
    std::fs::write(&file, b"x").unwrap();
    record_export(&lib, &file, "Vol 1", "kindle-paperwhite").unwrap();
    record_export(&lib, &file, "Vol 1", "kindle-paperwhite").unwrap();
    record_sent(&lib, &file, "Vol 1", SendVia::Usb, "Kindle").unwrap();
    let rows = list_exports(&lib, 10).unwrap();
    assert_eq!(rows.len(), 2);
    let (latest, older) = (&rows[0], &rows[1]);
    assert_eq!(latest.sent_via, Some(SendVia::Usb));
    assert_eq!(latest.sent_to.as_deref(), Some("Kindle"));
    assert!(latest.sent_at.is_some() && latest.exported_at.is_some());
    assert!(latest.exists);
    assert_eq!(older.sent_at, None);
    assert_eq!(latest.device, "kindle-paperwhite");
}

#[test]
fn sending_an_unknown_file_adds_a_row() {
    let lib = library::open_in_memory().unwrap();
    let file = Path::new("/nowhere/other.epub");
    record_sent(&lib, file, "other", SendVia::Email, "reader@kindle.com").unwrap();
    let rows = list_exports(&lib, 10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].exported_at, None);
    assert_eq!(rows[0].sent_via, Some(SendVia::Email));
    assert!(!rows[0].exists);
}

#[test]
fn list_is_newest_first_and_limited() {
    let lib = library::open_in_memory().unwrap();
    for i in 0..5 {
        record_export(&lib, Path::new(&format!("/b/{i}.epub")), "t", "d").unwrap();
    }
    let rows = list_exports(&lib, 3).unwrap();
    let files: Vec<&str> = rows.iter().map(|r| r.file.as_str()).collect();
    assert_eq!(files, ["/b/4.epub", "/b/3.epub", "/b/2.epub"]);
}

#[test]
fn backups_drop_the_smtp_password_and_restore_keeps_ours() {
    let settings = |password: &str, host: &str| Settings {
        kindle_email: EmailSettings {
            host: host.into(),
            password: password.into(),
            ..Default::default()
        },
        ..Settings::default()
    };
    let saved = sanitize(&settings("secret", "smtp.a.test"));
    assert_eq!(saved.kindle_email.password, "");
    assert_eq!(saved.kindle_email.host, "smtp.a.test");
    let merged = merge_settings(&settings("mine", "smtp.b.test"), &saved);
    assert_eq!(merged.kindle_email.password, "mine");
    assert_eq!(merged.kindle_email.host, "smtp.a.test");
}

#[test]
fn relative_paths_are_stored_absolute() {
    let lib = library::open_in_memory().unwrap();
    record_export(&lib, Path::new("out/vol-3.epub"), "Vol 3", "").unwrap();
    record_sent(
        &lib,
        Path::new("out/vol-4.epub"),
        "Vol 4",
        SendVia::Email,
        "r@kindle.com",
    )
    .unwrap();
    let cwd = std::env::current_dir().unwrap();
    let files: Vec<_> = list_exports(&lib, 10)
        .unwrap()
        .into_iter()
        .map(|r| r.file)
        .collect();
    let expect = |f: &str| cwd.join("out").join(f).to_string_lossy().into_owned();
    assert_eq!(files, vec![expect("vol-4.epub"), expect("vol-3.epub")]);
}
