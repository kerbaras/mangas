//! Which key signed an update, read from the signature alone (no bundle needed). A tauri
//! public key and signature are both base64 of a minisign file whose second line is
//! base64(algorithm[2] + key id[8] + …). The plugin still verifies the full signature when it
//! downloads; this only stops a release signed by another key from being offered at all.

use base64::{engine::general_purpose::STANDARD, Engine};

/// The minisign key id inside a base64 public key or signature.
pub fn key_id(encoded: &str) -> Option<[u8; 8]> {
    let file = String::from_utf8(STANDARD.decode(encoded.trim()).ok()?).ok()?;
    let raw = STANDARD.decode(file.lines().nth(1)?.trim()).ok()?;
    raw.get(2..10)?.try_into().ok()
}

/// Whether `signature` was made with the key `pubkey` is the public half of. An unreadable
/// signature or key never matches.
pub fn signed_by(signature: &str, pubkey: &str) -> bool {
    matches!((key_id(signature), key_id(pubkey)), (Some(a), Some(b)) if a == b)
}

/// `plugins.updater.pubkey` of a tauri config (`tauri.conf.json`).
pub fn pubkey_from_config(json: &str) -> Option<String> {
    let config: serde_json::Value = serde_json::from_str(json).ok()?;
    let key = config.pointer("/plugins/updater/pubkey")?.as_str()?;
    Some(key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = include_str!("../../../../apps/desktop/src-tauri/tauri.conf.json");

    fn signature(id: [u8; 8]) -> String {
        let raw = [b"ED".as_slice(), &id, &[0u8; 64]].concat();
        STANDARD.encode(format!(
            "untrusted comment: test\n{}\n",
            STANDARD.encode(raw)
        ))
    }

    fn pubkey() -> String {
        pubkey_from_config(CONFIG).unwrap_or_default()
    }

    #[test]
    fn reads_the_committed_public_key() {
        assert!(key_id(&pubkey()).is_some());
    }

    #[test]
    fn a_signature_by_the_configured_key_matches() {
        let Some(id) = key_id(&pubkey()) else {
            panic!("no key id in the committed pubkey");
        };
        assert!(signed_by(&signature(id), &pubkey()));
    }

    #[test]
    fn another_key_or_garbage_does_not() {
        assert!(!signed_by(&signature([7; 8]), &pubkey()));
        assert!(!signed_by("not base64!", &pubkey()));
        assert!(!signed_by(&STANDARD.encode("one line only"), &pubkey()));
        assert!(!signed_by(&signature([7; 8]), ""));
    }

    #[test]
    fn a_config_without_a_key_has_none() {
        assert_eq!(pubkey_from_config(r#"{"plugins":{}}"#), None);
        assert_eq!(pubkey_from_config("{"), None);
    }
}
