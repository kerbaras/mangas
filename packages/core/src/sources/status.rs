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

impl ProviderStatus {
    pub fn is_unreachable(&self) -> bool {
        matches!(self, Self::Unreachable { .. })
    }
}

/// Whether no provider can be used because none answered: every status is unreachable. A
/// partial outage, or an empty list, is not.
pub fn all_unreachable<'a>(statuses: impl IntoIterator<Item = &'a ProviderStatus>) -> bool {
    let mut statuses = statuses.into_iter().peekable();
    statuses.peek().is_some() && statuses.all(ProviderStatus::is_unreachable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_list_of_unreachable_providers_is_all_unreachable() {
        let down = || ProviderStatus::Unreachable { reason: "x".into() };
        assert!(all_unreachable(&[down(), down()]));
        assert!(!all_unreachable(&[down(), ProviderStatus::Ready]));
        assert!(!all_unreachable(&[down(), ProviderStatus::NeedsSession]));
        assert!(!all_unreachable(&[]));
    }
}
