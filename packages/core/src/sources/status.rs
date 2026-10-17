use std::fmt;

use serde::Serialize;

/// Whether a provider can be used right now, as reported by [`super::Source::status`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProviderStatus {
    Ready,
    /// The site wants a browser session (none stored, expired, or no longer accepted).
    NeedsSession,
    /// The site did not answer the probe: network error, timeout or an unexpected status.
    Unreachable {
        reason: String,
    },
}

impl fmt::Display for ProviderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready => f.write_str("ready"),
            Self::NeedsSession => f.write_str("needs a browser session"),
            Self::Unreachable { reason } => write!(f, "unreachable: {reason}"),
        }
    }
}
