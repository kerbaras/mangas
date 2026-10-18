//! A plugin call the capability doesn't grant fails silently in the UI, so check them together.

use std::{fs, path::Path};

use serde_json::Value;

const CAPABILITY: &str = include_str!("../capabilities/default.json");
const CONFIG: &str = include_str!("../tauri.conf.json");

/// What `opener:default` grants, per the plugin's `permissions/default.toml`.
const OPENER_DEFAULT: [&str; 2] = ["opener:allow-open-url", "opener:allow-reveal-item-in-dir"];

/// The UI-side opener functions and the permission each needs.
const CALLS: [(&str, &str); 3] = [
    ("openPath(", "opener:allow-open-path"),
    ("openUrl(", "opener:allow-open-url"),
    ("revealItemInDir(", "opener:allow-reveal-item-in-dir"),
];

fn granted() -> Vec<String> {
    let capability: Value = serde_json::from_str(CAPABILITY).unwrap();
    let mut out = Vec::new();
    for p in capability["permissions"].as_array().unwrap() {
        let id = p.as_str().or_else(|| p["identifier"].as_str()).unwrap();
        if id == "opener:default" {
            out.extend(OPENER_DEFAULT.map(String::from));
        }
        out.push(id.to_string());
    }
    out
}

fn ui_sources(dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() && !path.ends_with("test") {
            ui_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "ts" || e == "tsx") {
            out.push(fs::read_to_string(&path).unwrap());
        }
    }
}

#[test]
fn every_opener_call_in_the_ui_is_granted() {
    let mut sources = Vec::new();
    ui_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../src"),
        &mut sources,
    );
    let granted = granted();
    let used: Vec<_> = CALLS
        .iter()
        .filter(|(call, _)| sources.iter().any(|s| s.contains(call)))
        .collect();
    assert!(
        used.iter().any(|(call, _)| *call == "openPath("),
        "scan found no openPath call"
    );
    for (call, permission) in used {
        assert!(
            granted.iter().any(|g| g == permission),
            "{call} needs {permission}"
        );
    }
}

#[test]
fn open_path_is_scoped_and_reaches_dot_folders() {
    let capability: Value = serde_json::from_str(CAPABILITY).unwrap();
    let open_path = capability["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["identifier"] == "opener:allow-open-path")
        .unwrap();
    assert!(!open_path["allow"].as_array().unwrap().is_empty());
    // The data dir is `~/.local/share/mangas` on Linux, and `**` skips dot folders by default.
    let config: Value = serde_json::from_str(CONFIG).unwrap();
    assert_eq!(
        config["plugins"]["opener"]["requireLiteralLeadingDot"],
        false
    );
}
