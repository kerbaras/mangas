//! The provider contract, shared by the offline suite (`contract_suite.rs`, fixtures served by
//! wiremock) and the live smoke tests (`live.rs`). Providers are only used through the public
//! `Provider`/`Source` API, so every registered provider is checked the same way.
#![allow(dead_code)]

pub mod headers;
pub mod manifest;
pub mod walk;

use chrono::Utc;
use mangas_core::library::{ProviderSession, Settings};

/// Settings carrying a browser session for `id`, as the desktop app would store it.
pub fn with_session(id: &str) -> Settings {
    let mut settings = Settings::default();
    settings.provider_sessions.insert(id.to_string(), session());
    settings
}

pub fn session() -> ProviderSession {
    ProviderSession {
        cookies: "contract_clearance=1".into(),
        user_agent: "contract-agent/1.0".into(),
        saved_at: Some(Utc::now().to_rfc3339()),
    }
}
