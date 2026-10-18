//! The common layer every provider builds on: [`SiteClient`] (rate limit, timeouts, retries,
//! challenge detection, JSON helpers, status probe) and [`HostPolicy`] (page image headers).

mod client;
mod config;
mod host_policy;
mod response;
mod retry;
mod status;

use mangas_core::domain::{Error, Result};
use mangas_core::sources::Header;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub use client::SiteClient;
pub use config::ClientConfig;
pub use host_policy::{host_matches, HostPolicy};
pub use retry::{edge_error, parse_retry_after, retryable_status, RetryPolicy};
pub use status::{probe_status, session_gate};

pub const USER_AGENT: &str = concat!(
    "mangas/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/kerbaras/mangas)"
);

/// Status and body of a response; providers map their own error payloads.
pub struct Reply {
    pub status: u16,
    pub body: String,
}

/// Cloudflare marks challenge responses with `cf-mitigated: challenge`; older edges only send
/// the interstitial page with a 403.
pub fn is_challenge(status: u16, cf_mitigated: &str, body: &str) -> bool {
    cf_mitigated.eq_ignore_ascii_case("challenge")
        || (status == 403 && body.contains("challenge-platform"))
}

pub fn header_map(headers: &[Header]) -> Result<HeaderMap> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| Error::Other(format!("header {name}: {e}")))?;
        let value = HeaderValue::from_str(value)
            .map_err(|e| Error::Other(format!("header {name}: {e}")))?;
        map.insert(name, value);
    }
    Ok(map)
}
