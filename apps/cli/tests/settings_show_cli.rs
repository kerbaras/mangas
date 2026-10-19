//! `mangas settings show|proxy|concurrency` and the global `--proxy` flag.

use std::path::Path;
use std::process::{Command, Output};

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn show_marks_defaults_and_json_holds_only_changes() {
    let home = tempfile::tempdir().unwrap();
    let shown = stdout(&mangas(home.path(), &["settings", "show"]));
    assert!(
        shown.contains("download concurrency  4 (default)"),
        "{shown}"
    );
    assert!(
        shown.contains("proxy                 system (default)"),
        "{shown}"
    );
    for row in shown.lines() {
        assert!(
            row.ends_with("default)"),
            "a fresh library shows every row at its default: {row}"
        );
    }
    assert_eq!(
        stdout(&mangas(home.path(), &["settings", "show", "--json"])),
        "{}\n"
    );

    assert_eq!(
        stdout(&mangas(home.path(), &["settings", "concurrency", "6"])),
        "6\n"
    );
    let set = mangas(
        home.path(),
        &["settings", "proxy", "http://me:pw@127.0.0.1:8888"],
    );
    assert_eq!(stdout(&set), "http://me:***@127.0.0.1:8888/\n");
    let shown = stdout(&mangas(home.path(), &["settings", "show"]));
    assert!(shown.contains("download concurrency  6\n"), "{shown}");
    assert!(
        shown.contains("proxy                 http://me:***@127.0.0.1:8888/"),
        "{shown}"
    );
    let json: serde_json::Value = serde_json::from_str(&stdout(&mangas(
        home.path(),
        &["settings", "show", "--json"],
    )))
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "download_concurrency": 6,
            "proxy_mode": "custom",
            "proxy_url": "http://me:***@127.0.0.1:8888/"
        })
    );
}

#[test]
fn bad_values_are_refused_and_nothing_is_saved() {
    let home = tempfile::tempdir().unwrap();
    assert!(!mangas(home.path(), &["settings", "concurrency", "9"])
        .status
        .success());
    let bad = mangas(home.path(), &["settings", "proxy", "socks5://p:1"]);
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("use http:// or https://"));
    assert_eq!(
        stdout(&mangas(home.path(), &["settings", "proxy"])),
        "system\n"
    );
    let flag = mangas(home.path(), &["--proxy", "ftp://x", "settings", "show"]);
    assert!(!flag.status.success());
}

#[test]
fn a_dead_proxy_flag_names_the_proxy() {
    let home = tempfile::tempdir().unwrap();
    let out = mangas(
        home.path(),
        &[
            "--proxy",
            "http://127.0.0.1:9",
            "providers",
            "check",
            "mangadex",
        ],
    );
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("couldn't connect through the proxy http://127.0.0.1:9"),
        "{text}"
    );
}

#[test]
fn show_lists_the_replace_switch_the_app_saves() {
    use mangas_core::library::{self, Settings};
    let home = tempfile::tempdir().unwrap();
    let shown = stdout(&mangas(home.path(), &["settings", "show"]));
    assert!(
        shown.contains("export replace        off (default)\n"),
        "{shown}"
    );
    let db = home.path().join("data/mangas/mangas.db");
    let lib = library::open(&db).unwrap();
    let on = Settings {
        export_replace: Some(true),
        ..library::get_settings(&lib).unwrap()
    };
    library::edit_settings(&lib, None, &on).unwrap();
    let shown = stdout(&mangas(home.path(), &["settings", "show"]));
    assert!(shown.contains("export replace        on\n"), "{shown}");
    let json = stdout(&mangas(home.path(), &["settings", "show", "--json"]));
    assert!(json.contains("\"export_replace\": true"), "{json}");
}
