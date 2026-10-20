//! The updater manifest (`latest.json`) and what it means for the running app.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{is_older, parse_version, should_update, signed_by, Channel};
use crate::domain::{Error, Result};

/// `latest.json` as the release workflow writes it (the `tauri-plugin-updater` format).
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub pub_date: Option<String>,
    /// Signed bundles by target: `linux-x86_64`, `linux-x86_64-deb`, `darwin-aarch64`, …
    #[serde(default)]
    pub platforms: BTreeMap<String, serde_json::Value>,
}

/// Result of a check, for the app's About section and `mangas update --check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Status {
    UpToDate {
        current: String,
        channel: Channel,
    },
    Available {
        current: String,
        version: String,
        notes: Option<String>,
        date: Option<String>,
        channel: Channel,
    },
    /// The server offers an older version than the running one: nothing to install.
    Older {
        current: String,
        version: String,
        channel: Channel,
    },
    /// Newer, but not signed by the key the app trusts: never offered.
    InvalidSignature {
        current: String,
        version: String,
        channel: Channel,
    },
}

impl Status {
    /// "You're up to date" / "mangas 0.5.0 is available"; same wording as the app.
    pub fn line(&self) -> String {
        match self {
            Self::UpToDate { current, channel } => {
                format!(
                    "You're up to date (v{current}, {} channel)",
                    channel.label()
                )
            }
            Self::Available {
                current, version, ..
            } => format!("mangas v{version} is available (you have v{current})"),
            Self::Older {
                current, version, ..
            } => format!(
                "The server offers v{version}, older than your v{current}: nothing to install"
            ),
            Self::InvalidSignature { version, .. } => format!(
                "v{version} isn't signed with the key this app trusts, so it won't be installed"
            ),
        }
    }

    /// A check that found something wrong with the release (the CLI exits non-zero).
    pub fn is_refusal(&self) -> bool {
        matches!(self, Self::InvalidSignature { .. })
    }
}

pub fn parse_manifest(json: &str) -> Result<Manifest> {
    serde_json::from_str(json).map_err(|e| Error::Other(format!("update manifest: {e}")))
}

/// The manifest key of this build: `{os}-{arch}` with tauri's names (`darwin`, `x86_64`).
pub fn current_target() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    format!("{os}-{}", std::env::consts::ARCH)
}

/// What `manifest` means for `current` on `channel`. A newer release without a bundle for
/// `target` is an error rather than "up to date", so a missing build shows up; one whose
/// bundles for `target` aren't all signed by `pubkey` is [`Status::InvalidSignature`].
pub fn evaluate(
    current: &str,
    manifest: &Manifest,
    channel: Channel,
    target: &str,
    pubkey: &str,
) -> Result<Status> {
    let bad = |what: &str, e: semver::Error| Error::Other(format!("{what} version: {e}"));
    let installed = parse_version(current).map_err(|e| bad("app", e))?;
    let remote = parse_version(&manifest.version).map_err(|e| bad("release", e))?;
    let (current, version) = (installed.to_string(), remote.to_string());
    if is_older(&installed, &remote) {
        return Ok(Status::Older {
            current,
            version,
            channel,
        });
    }
    if !should_update(&installed, &remote, channel) {
        return Ok(Status::UpToDate { current, channel });
    }
    let prefix = format!("{target}-");
    let builds: Vec<_> = (manifest.platforms.iter())
        .filter(|(k, _)| *k == target || k.starts_with(&prefix))
        .map(|(_, build)| build.get("signature").and_then(|s| s.as_str()))
        .collect();
    if builds.is_empty() {
        return Err(Error::Other(format!("v{remote} has no build for {target}")));
    }
    if !builds
        .iter()
        .all(|s| s.is_some_and(|s| signed_by(s, pubkey)))
    {
        return Ok(Status::InvalidSignature {
            current,
            version,
            channel,
        });
    }
    Ok(Status::Available {
        current,
        version,
        notes: manifest.notes.clone().filter(|n| !n.trim().is_empty()),
        date: manifest.pub_date.clone(),
        channel,
    })
}
