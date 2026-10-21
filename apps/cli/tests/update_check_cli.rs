//! `mangas update --check` when the release feed can't be reached: the line names the cause
//! (#316), once, in text and `--json`.

use std::path::Path;
use std::process::{Command, Output};

fn check(home: &Path, url: &str, json: bool) -> Output {
    let mut args = vec!["update", "--check"];
    if json {
        args.push("--json");
    }
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("MANGAS_UPDATE_URL", url)
        .env("NO_PROXY", "*")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

/// A local port nothing listens on.
fn refused_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{addr}/latest.json")
}

#[test]
fn a_refused_port_names_the_cause_once() {
    let home = tempfile::tempdir().unwrap();
    let url = refused_url();
    let text = check(home.path(), &url, false);
    assert_eq!(text.status.code(), Some(1), "{text:?}");
    let stderr = String::from_utf8_lossy(&text.stderr);
    assert!(
        stderr.starts_with("error: couldn't check for updates: network: "),
        "{stderr}"
    );
    assert!(stderr.contains(&url), "{stderr}");
    assert_eq!(stderr.matches("Connection refused").count(), 1, "{stderr}");

    let json = check(home.path(), &url, true);
    assert_eq!(json.status.code(), Some(1), "{json:?}");
    let line = String::from_utf8_lossy(&json.stdout);
    let report: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(report["kind"], "error");
    let message = report["message"].as_str().unwrap_or_default();
    assert_eq!(
        message.matches("Connection refused").count(),
        1,
        "{message}"
    );
    assert!(
        stderr.trim_end().ends_with(message),
        "{stderr} vs {message}"
    );
}
