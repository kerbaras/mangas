//! Release channels and the URLs their update manifests are published at.

use serde::{Deserialize, Serialize};

/// Where releases are published.
pub const RELEASES: &str = "https://github.com/kerbaras/mangas/releases";

/// Tag of the rolling pre-release the release workflow keeps the channel manifests on.
pub const CHANNELS_TAG: &str = "updater";

/// Environment variable that replaces every channel URL with one manifest URL (a local test
/// server or a mirror). Installs still need the signature of the key built into the app.
pub const OVERRIDE_ENV: &str = "MANGAS_UPDATE_URL";

/// Which releases count as updates: only full releases, or release candidates as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Rc,
}

impl Channel {
    /// `stable` or `rc`, as stored and typed on the command line.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "stable" => Some(Self::Stable),
            "rc" => Some(Self::Rc),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Rc => "release candidates",
        }
    }
}

/// Manifest URLs for `channel`, tried in order until one answers. The channel files come
/// first; the fallbacks cover releases made before the channel release existed. The stable
/// list never reads an rc manifest: GitHub's `latest` release is never a pre-release.
pub fn endpoints(channel: Channel, override_url: Option<&str>) -> Vec<String> {
    if let Some(url) = override_url.map(str::trim).filter(|u| !u.is_empty()) {
        return vec![url.to_string()];
    }
    let rolling = |name: &str| format!("{RELEASES}/download/{CHANNELS_TAG}/{name}");
    let latest = format!("{RELEASES}/latest/download/latest.json");
    match channel {
        Channel::Stable => vec![rolling("latest.json"), latest],
        Channel::Rc => vec![rolling("latest-rc.json"), rolling("latest.json"), latest],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_never_reads_the_rc_manifest() {
        let urls = endpoints(Channel::Stable, None);
        assert_eq!(urls[0], format!("{RELEASES}/download/updater/latest.json"));
        assert!(urls.iter().all(|u| !u.contains("rc")));
    }

    #[test]
    fn rc_reads_its_own_manifest_first_then_stable() {
        let urls = endpoints(Channel::Rc, None);
        assert!(urls[0].ends_with("/updater/latest-rc.json"));
        assert!(urls[1].ends_with("/updater/latest.json"));
    }

    #[test]
    fn an_override_replaces_every_channel_url() {
        let url = "http://127.0.0.1:8000/latest.json";
        assert_eq!(endpoints(Channel::Rc, Some(url)), vec![url]);
        assert_eq!(endpoints(Channel::Stable, Some("  ")).len(), 2);
    }

    #[test]
    fn channels_parse_and_serialise_lowercase() {
        assert_eq!(Channel::parse(" RC "), Some(Channel::Rc));
        assert_eq!(Channel::parse("beta"), None);
        assert_eq!(
            serde_json::to_string(&Channel::Rc).ok().as_deref(),
            Some("\"rc\"")
        );
    }
}
