//! Folders open through the `open_folder` command, which checks the path; the webview itself
//! must not be able to open arbitrary paths with the opener plugin.

use std::{fs, path::Path};

const CAPABILITY: &str = include_str!("../capabilities/default.json");

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
