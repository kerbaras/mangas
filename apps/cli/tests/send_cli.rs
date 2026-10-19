//! `mangas send` and `mangas settings email` against a fake Kindle mount and a local SMTP
//! double.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

mod support;
use support::smtp_double;

fn mangas(home: &Path, args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn settings_email_saves_and_masks_the_password() {
    let home = tempfile::tempdir().unwrap();
    let args = [
        "settings",
        "email",
        "--to",
        "reader@kindle.com",
        "--host",
        "smtp.example.test",
        "--security",
        "tls",
        "--username",
        "me@example.test",
        "--password-stdin",
    ];
    let out = mangas(home.path(), &args, "not-a-real-password\n");
    assert!(out.status.success(), "{out:?}");
    let shown = text(&out.stdout);
    assert!(
        shown.contains("server    smtp.example.test:465 (tls)"),
        "{shown}"
    );
    assert!(shown.contains("from      me@example.test"), "{shown}");
    assert!(
        shown.contains("password  (set, unencrypted in the app settings)"),
        "{shown}"
    );
    assert!(!shown.contains("not-a-real-password"), "{shown}");
    let warning = text(&out.stderr);
    assert!(
        warning.contains(
            "the SMTP password is stored unencrypted in the app settings \
             (system keychain turned off by MANGAS_KEYCHAIN)"
        ),
        "{warning}"
    );
    let again = text(&mangas(home.path(), &["settings", "email"], "").stdout);
    assert_eq!(again, shown);
}

#[test]
fn send_usb_copies_to_the_given_mount() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let book = home.path().join("Vol 1.epub");
    std::fs::write(&book, b"epub").unwrap();
    let out = mangas(
        home.path(),
        &[
            "send",
            book.to_str().unwrap(),
            "--via",
            "usb",
            "--kindle",
            kindle.to_str().unwrap(),
        ],
        "",
    );
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        std::fs::read(kindle.join("documents/Vol 1.epub")).unwrap(),
        b"epub"
    );
    assert!(text(&out.stdout).contains("copied to"));
}

#[test]
fn send_usb_without_a_device_says_so() {
    let home = tempfile::tempdir().unwrap();
    let book = home.path().join("a.epub");
    std::fs::write(&book, b"x").unwrap();
    let empty = home.path().join("not-a-kindle");
    std::fs::create_dir_all(&empty).unwrap();
    let out = mangas(
        home.path(),
        &[
            "send",
            book.to_str().unwrap(),
            "--via",
            "usb",
            "--kindle",
            empty.to_str().unwrap(),
        ],
        "",
    );
    assert!(!out.status.success());
    assert!(text(&out.stderr).contains("Device not found"), "{out:?}");
}

#[test]
fn send_email_and_test_go_through_smtp() {
    let home = tempfile::tempdir().unwrap();
    let (port, session) = smtp_double();
    let port = port.to_string();
    let setup = [
        "settings",
        "email",
        "--to",
        "reader@kindle.com",
        "--from",
        "me@example.test",
        "--host",
        "127.0.0.1",
        "--port",
        &port,
        "--security",
        "none",
    ];
    assert!(mangas(home.path(), &setup, "").status.success());
    let book = home.path().join("Vol 2.epub");
    std::fs::write(&book, b"epub").unwrap();
    let out = mangas(home.path(), &["send", book.to_str().unwrap()], "");
    assert!(out.status.success(), "{out:?}");
    assert!(text(&out.stdout).contains("to reader@kindle.com"));
    let log = session.join().unwrap();
    assert!(log.contains("RCPT TO:<reader@kindle.com>"), "{log}");
    assert!(log.contains("filename=\"Vol 2.epub\""), "{log}");

    let (port, session) = smtp_double();
    let port = port.to_string();
    let reset = ["settings", "email", "--port", &port];
    assert!(mangas(home.path(), &reset, "").status.success());
    let out = mangas(home.path(), &["send", "--test"], "");
    assert!(out.status.success(), "{out:?}");
    assert!(session
        .join()
        .unwrap()
        .contains("RCPT TO:<me@example.test>"));
}

#[test]
fn kindle_send_reports_a_failed_send_with_exit_code_2() {
    let home = tempfile::tempdir().unwrap();
    let chapter = home.path().join("Series/ch1");
    std::fs::create_dir_all(&chapter).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(chapter.join("001.png")).unwrap();
    let out_file = home.path().join("out.epub");
    let out = mangas(
        home.path(),
        &[
            "kindle",
            chapter.to_str().unwrap(),
            "-o",
            out_file.to_str().unwrap(),
            "--send",
            "email",
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out_file.is_file());
    let err = text(&out.stderr);
    assert!(err.contains("was written but not sent"), "{err}");
    assert!(err.contains("Email delivery isn't set up"), "{err}");
}
