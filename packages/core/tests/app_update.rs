use base64::{engine::general_purpose::STANDARD, Engine};
use mangas_core::app_update::{
    evaluate, key_id, parse_manifest, pubkey_from_config, Channel, Status,
};

const CONFIG: &str = include_str!("../../../apps/desktop/src-tauri/tauri.conf.json");

fn pubkey() -> String {
    pubkey_from_config(CONFIG).unwrap_or_default()
}

/// A signature carrying `id` as its key id (the bytes after it don't matter here).
fn signature(id: [u8; 8]) -> String {
    let raw = [b"ED".as_slice(), &id, &[0u8; 64]].concat();
    STANDARD.encode(format!(
        "untrusted comment: test\n{}\n",
        STANDARD.encode(raw)
    ))
}

fn manifest(version: &str, id: [u8; 8]) -> String {
    let sig = signature(id);
    serde_json::json!({
        "version": version,
        "notes": "Auto-updater",
        "pub_date": "2026-10-18T12:00:00Z",
        "platforms": {
            "linux-x86_64": { "signature": sig, "url": "https://example.test/a.AppImage" },
            "linux-x86_64-deb": { "signature": sig, "url": "https://example.test/a.deb" },
            "darwin-aarch64-app": { "signature": sig, "url": "https://example.test/a.app.tar.gz" }
        }
    })
    .to_string()
}

fn trusted() -> [u8; 8] {
    key_id(&pubkey()).unwrap_or_else(|| panic!("no key id in the committed pubkey"))
}

fn check_with(json: &str, current: &str, channel: Channel, target: &str) -> Result<Status, String> {
    let manifest = parse_manifest(json).map_err(|e| e.to_string())?;
    evaluate(current, &manifest, channel, target, &pubkey()).map_err(|e| e.to_string())
}

fn check(current: &str, channel: Channel, target: &str) -> Result<Status, String> {
    check_with(&manifest("0.5.0-rc.1", trusted()), current, channel, target)
}

#[test]
fn an_rc_manifest_is_an_update_on_the_rc_channel() {
    let Ok(Status::Available { version, notes, .. }) = check("0.4.0", Channel::Rc, "linux-x86_64")
    else {
        panic!("expected an update");
    };
    assert_eq!(
        (version.as_str(), notes.as_deref()),
        ("0.5.0-rc.1", Some("Auto-updater"))
    );
}

#[test]
fn an_rc_manifest_is_up_to_date_on_stable() {
    let status = check("0.4.0", Channel::Stable, "linux-x86_64");
    assert!(matches!(status, Ok(Status::UpToDate { .. })));
}

#[test]
fn the_same_version_is_up_to_date() {
    let status = check("0.5.0-rc.1", Channel::Rc, "linux-x86_64");
    assert!(matches!(status, Ok(Status::UpToDate { .. })));
}

#[test]
fn an_older_release_is_its_own_outcome() {
    let status = check_with(
        &manifest("0.3.0", trusted()),
        "0.3.9",
        Channel::Rc,
        "linux-x86_64",
    );
    let Ok(older @ Status::Older { .. }) = status else {
        panic!("expected older, got {status:?}");
    };
    assert_eq!(
        older.line(),
        "The server offers v0.3.0, older than your v0.3.9: nothing to install"
    );
    assert!(!older.is_refusal());
    let json = serde_json::to_value(&older).unwrap_or_default();
    assert_eq!(
        (json["kind"].as_str(), json["version"].as_str()),
        (Some("older"), Some("0.3.0"))
    );
}

#[test]
fn a_release_signed_by_another_key_is_refused() {
    let status = check_with(
        &manifest("0.5.0", [7; 8]),
        "0.4.0",
        Channel::Stable,
        "linux-x86_64",
    );
    let Ok(refused @ Status::InvalidSignature { .. }) = status else {
        panic!("expected invalid_signature, got {status:?}");
    };
    assert!(refused.is_refusal());
    assert_eq!(
        refused.line(),
        "v0.5.0 isn't signed with the key this app trusts, so it won't be installed"
    );
    let json = serde_json::to_value(&refused).unwrap_or_default();
    assert_eq!(json["kind"].as_str(), Some("invalid_signature"));
}

#[test]
fn an_unreadable_signature_is_refused() {
    let json = manifest("0.5.0", trusted()).replace(&signature(trusted()), "s");
    let status = check_with(&json, "0.4.0", Channel::Stable, "linux-x86_64");
    assert!(matches!(status, Ok(Status::InvalidSignature { .. })));
}

#[test]
fn a_bundle_suffix_counts_as_a_build_for_the_target() {
    assert!(matches!(
        check("0.4.0", Channel::Rc, "darwin-aarch64"),
        Ok(Status::Available { .. })
    ));
}

#[test]
fn a_newer_release_without_a_build_for_this_target_is_an_error() {
    let error = check("0.4.0", Channel::Rc, "windows-x86_64");
    assert_eq!(
        error,
        Err("v0.5.0-rc.1 has no build for windows-x86_64".into())
    );
}

#[test]
fn a_broken_manifest_or_version_is_an_error() {
    assert!(parse_manifest("<html>").is_err());
    assert!(check("dev", Channel::Rc, "linux-x86_64").is_err());
}

#[test]
fn status_lines_match_the_app() {
    let up = Status::UpToDate {
        current: "0.5.0".into(),
        channel: Channel::Stable,
    };
    assert_eq!(up.line(), "You're up to date (v0.5.0, stable channel)");
    let Ok(available) = check("0.4.0", Channel::Rc, "linux-x86_64") else {
        panic!("expected an update");
    };
    assert_eq!(
        available.line(),
        "Mangas v0.5.0-rc.1 is available (you have v0.4.0)"
    );
}
