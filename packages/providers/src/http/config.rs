use std::time::Duration;

use super::RetryPolicy;
use crate::provider::ProviderInfo;

/// How a [`super::SiteClient`] talks to one site. Start from [`ClientConfig::new`] and override
/// fields with struct update syntax:
/// `ClientConfig { requests_per_second: 5, ..ClientConfig::new(INFO, API_BASE) }`.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Provider id, site (referer, challenge message) and whether a session is required.
    pub info: ProviderInfo,
    /// Prefix of every path passed to the client.
    pub base: String,
    /// Hosts serving page images that need the site's referer (exact host or any subdomain;
    /// `"*"` means every host). The site's own host is always included.
    pub image_hosts: &'static [&'static str],
    pub requests_per_second: u32,
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub retry: RetryPolicy,
    /// Known lifetime of the browser session; older sessions report `NeedsSession`.
    pub session_ttl: Option<Duration>,
}

impl ClientConfig {
    pub fn new(info: ProviderInfo, base: &str) -> Self {
        Self {
            info,
            base: base.trim_end_matches('/').to_string(),
            image_hosts: &[],
            requests_per_second: 3,
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            retry: RetryPolicy::default(),
            session_ttl: None,
        }
    }
}
