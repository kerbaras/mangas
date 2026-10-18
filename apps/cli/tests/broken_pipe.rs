//! Piping into `head` closes stdout early; the CLI must exit 0 without a panic message.

#![cfg(unix)]

use std::process::{Command, Stdio};

#[test]
fn closed_stdout_exits_quietly() {
    let home = tempfile::tempdir().unwrap();
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let output = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(["kindle", "--list-devices"])
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit {:?}: {stderr}",
        output.status
    );
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
}
