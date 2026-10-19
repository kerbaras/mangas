//! Provider status as pure functions of the stored session and one probe response.

use std::time::Duration;

use chrono::{DateTime, Utc};
use mangas_core::domain::{Error, Result};
use mangas_core::sources::ProviderStatus;

use crate::session::Session;

/// The session's verdict: a required session that is missing or past its TTL. It holds only once
/// the host answers ([`answered_gate`]).
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

/// A session gate holds only once the host has answered (a reply or a verification page): a host
/// that can't be reached is unreachable whether or not a session is saved.
pub fn answered_gate<T>(gate: Option<ProviderStatus>, reply: &Result<T>) -> Option<ProviderStatus> {
    match reply {
        Ok(_) | Err(Error::Challenge { .. }) => gate,
        Err(e) => Some(ProviderStatus::Unreachable {
            reason: e.to_string(),
        }),
    }
}
