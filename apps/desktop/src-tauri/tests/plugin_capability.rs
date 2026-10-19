//! Plugin calls the capability doesn't grant fail silently in the webview, so check them here.
//! Folders open through the `open_folder` command, which checks the path; the webview itself
//! must not be able to open arbitrary paths with the opener plugin.

use std::{fs, path::Path};

const CAPABILITY: &str = include_str!("../capabilities/default.json");
const SETUP: &str = include_str!("../src/lib.rs");
const WINDOW_STATE: &str = include_str!("../src/window_state.rs");

/// Commands a plugin's injected init script calls on every page load, by plugin.
const INIT_SCRIPT_CALLS: [(&str, &str); 1] =
    [("notification", "notification:allow-is-permission-granted")];

fn ui_sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() && !path.ends_with("test") {
            ui_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "ts" || e == "tsx") {
            out.push((
                path.display().to_string(),
                fs::read_to_string(&path).unwrap(),
            ));
        }
    }
}

#[test]
fn the_webview_cannot_open_paths_directly() {
    assert!(!CAPABILITY.contains("opener:allow-open-path"));
    assert!(!CAPABILITY.contains("opener:allow-reveal-item-in-dir"));
}

#[test]
fn the_ui_opens_folders_only_through_open_folder() {
    let mut sources = Vec::new();
    ui_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../src"),
        &mut sources,
    );
    for (file, source) in &sources {
        for call in ["openPath(", "revealItemInDir("] {
            assert!(
                !source.contains(call),
                "{file} calls {call}; use openFolder"
            );
        }
    }
    assert!(sources.iter().any(|(_, s)| s.contains("\"open_folder\"")));
}

/// `clipboard-manager` from `@tauri-apps/plugin-clipboard-manager"` in each source.
fn imported_plugins(sources: &[(String, String)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, source) in sources {
        for part in source.split("@tauri-apps/plugin-").skip(1) {
            let name: String = part
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

#[test]
fn every_plugin_the_ui_imports_has_a_permission() {
    let mut sources = Vec::new();
    ui_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../src"),
        &mut sources,
    );
    let plugins = imported_plugins(&sources);
    assert!(
        plugins.iter().any(|p| p == "dialog"),
        "scan found no plugin imports"
    );
    for plugin in plugins {
        assert!(
            CAPABILITY.contains(&format!("\"{plugin}:")),
            "the UI imports @tauri-apps/plugin-{plugin} but the capability grants nothing for it"
        );
    }
}

#[test]
fn plugins_with_init_scripts_get_what_the_script_calls() {
    for (plugin, permission) in INIT_SCRIPT_CALLS {
        let registered = SETUP.contains(&format!("tauri_plugin_{plugin}::"));
        assert!(
            !registered || CAPABILITY.contains(permission),
            "{plugin}'s init script needs {permission}"
        );
    }
}

#[test]
fn the_window_state_plugin_is_granted_in_one_place() {
    assert!(SETUP.contains("window_state::plugin()"));
    assert!(WINDOW_STATE.contains("tauri_plugin_window_state::Builder"));
    assert_eq!(CAPABILITY.matches("\"window-state:").count(), 1);
    assert!(CAPABILITY.contains("\"window-state:default\""));
}
