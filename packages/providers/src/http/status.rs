//! Provider status as pure functions of the stored session and one probe response.

use std::time::Duration;

use chrono::{DateTime, Utc};
use mangas_core::domain::{Error, Result};
use mangas_core::sources::ProviderStatus;

use crate::session::Session;

/// The status known without a request: a required session that is missing or past its TTL.
pub fn session_gate(
    needs_session: bool,
    session: Option<&Session>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,
) -> Option<ProviderStatus> {
    match session {
        None if needs_session => Some(ProviderStatus::NeedsSession),
        Some(s) if ttl.is_some_and(|ttl| s.is_expired(ttl, now)) => {
            Some(ProviderStatus::NeedsSession)
        }
        _ => None,
    }
}

/// The status a probe response implies.
pub fn probe_status<T>(result: &Result<T>) -> ProviderStatus {
    match result {
        Ok(_) => ProviderStatus::Ready,
        Err(Error::Challenge { .. }) => ProviderStatus::NeedsSession,
        Err(e) => ProviderStatus::Unreachable {
            reason: e.to_string(),
        },
    }
}
