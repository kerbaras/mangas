//! #330: every `mangas send --json` exit prints one JSON object on stdout with the `eject`
//! object, also without a book and when it fails. A fake Kindle (a plain folder) is never a
//! drive of its own, so its eject is refused; `MANGAS_TEST_KINDLE` (a mounted Kindle volume
//! with a book in `documents/`) runs the busy and the ejected case on a real drive.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::{json, Value};

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_KEYCHAIN", "off")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

/// stdout as the one JSON object it must be, the exit failed.
fn failed_json(out: &Output) -> Value {
    assert!(!out.status.success(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

fn not_attempted() -> Value {
    json!({ "attempted": false, "ok": false, "error": null })
}

#[test]
fn eject_only_json_reports_a_refused_eject() {
    let home = tempfile::tempdir().unwrap();
    let kindle = home.path().join("Kindle");
    std::fs::create_dir_all(kindle.join("documents")).unwrap();
    let k = kindle.to_str().unwrap();
    let out = mangas(
        home.path(),
        &["send", "--via", "usb", "--eject", "--json", "--kindle", k],
    );
    let report = failed_json(&out);
    assert_eq!(report["file"], Value::Null, "{report}");
    assert_eq!(report["via"], "usb", "{report}");
    assert_eq!(report["sent"], false, "{report}");
    assert_eq!(report["eject"]["attempted"], true, "{report}");
    assert_eq!(report["eject"]["ok"], false, "{report}");
    let error = report["eject"]["error"].as_str().unwrap_or_default();
    assert!(error.contains("isn't a drive of its own"), "{report}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("can't eject"),
        "{out:?}"
    );
}

#[test]
fn eject_only_json_by_email_is_refused_as_json() {
    let home = tempfile::tempdir().unwrap();
    let report = failed_json(&mangas(home.path(), &["send", "--eject", "--json"]));
    assert_eq!(report["via"], "email", "{report}");
    assert_eq!(report["eject"], not_attempted(), "{report}");
    let error = report["error"].as_str().unwrap_or_default();
    assert!(error.contains("add --via usb"), "{report}");
}

#[test]
fn send_json_without_a_book_or_with_a_missing_one_is_json() {
    let home = tempfile::tempdir().unwrap();
    let report = failed_json(&mangas(home.path(), &["send", "--json"]));
    assert!(report["error"]
        .as_str()
        .unwrap_or_default()
        .contains("no book given"));
    assert_eq!(report["eject"], not_attempted(), "{report}");
    let missing = home.path().join("gone.epub");
    let m = missing.to_str().unwrap();
    let args = ["send", "--via", "usb", "--eject", "--json", m];
    let report = failed_json(&mangas(home.path(), &args));
    assert_eq!(report["file"], m, "{report}");
    assert!(report["error"]
        .as_str()
        .unwrap_or_default()
        .contains("is not a file"));
    assert_eq!(report["eject"], not_attempted(), "{report}");
}

/// Busy while a program holds a book open, then ejected once it closes. Needs a mounted Kindle
/// volume you can lose: `MANGAS_TEST_KINDLE=/media/$USER/KINDLE cargo test -- --ignored`.
#[test]
#[ignore = "needs MANGAS_TEST_KINDLE, a mounted Kindle volume"]
fn eject_only_json_on_a_real_drive() {
    let Some(mount) = std::env::var_os("MANGAS_TEST_KINDLE") else {
        return;
    };
    let home = tempfile::tempdir().unwrap();
    let k = mount.to_str().unwrap();
    let book = Path::new(k).join("documents/held.epub");
    std::fs::write(&book, b"held").unwrap();
    let args = ["send", "--via", "usb", "--eject", "--json", "--kindle", k];
    let mut holder = Command::new("tail").arg("-f").arg(&book).spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let busy = failed_json(&mangas(home.path(), &args));
    drop(holder.kill());
    drop(holder.wait());
    assert_eq!(busy["eject"]["attempted"], true, "{busy}");
    assert_eq!(busy["eject"]["ok"], false, "{busy}");
    assert!(busy["eject"]["error"]
        .as_str()
        .unwrap_or_default()
        .contains("in use by tail"));
    let out = mangas(home.path(), &args);
    assert!(out.status.success(), "{out:?}");
    let done: Value = serde_json::from_slice(&out.stdout).unwrap();
    let ok = json!({ "attempted": true, "ok": true, "error": null });
    assert_eq!(done["eject"], ok, "{done}");
    assert!(done["to"].is_string(), "{done}");
    assert!(
        !Path::new(k).join("documents").exists(),
        "{k} is still mounted"
    );
}
