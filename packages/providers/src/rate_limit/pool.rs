use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use super::TokenBucket;

/// Token buckets keyed by host. Clients ask for their host's bucket instead of owning one, so
/// two clients (or two sources built by separate calls) for the same host share one budget.
/// When clients declare different rates for one host, the strictest one applies.
#[derive(Default)]
pub struct HostLimiters {
    buckets: Mutex<HashMap<String, Arc<TokenBucket>>>,
}

impl HostLimiters {
    /// The process-wide pool every [`crate::http::SiteClient`] uses.
    pub fn shared() -> &'static HostLimiters {
        static SHARED: OnceLock<HostLimiters> = OnceLock::new();
        SHARED.get_or_init(HostLimiters::default)
    }

    /// The bucket for the host of `url`, created on first use with a burst of
    /// `requests_per_second` and tightened if a later caller asks for less.
    pub fn bucket(&self, url: &str, requests_per_second: u32) -> Arc<TokenBucket> {
        let rate = requests_per_second.max(1);
        let mut buckets = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);
        let bucket = buckets
            .entry(host_key(url))
            .or_insert_with(|| Arc::new(TokenBucket::new(rate, rate)));
        bucket.tighten(rate, rate);
        Arc::clone(bucket)
    }
}

/// `host:port` of `url` (default port filled in), the unit a site rate-limits by. Unparsable
/// input is used as is, so it still gets a bucket of its own.
pub fn host_key(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(parsed) => match (parsed.host_str(), parsed.port_or_known_default()) {
            (Some(host), Some(port)) => format!("{host}:{port}"),
            (Some(host), None) => host.to_string(),
            _ => url.to_string(),
        },
        Err(_) => url.to_string(),
    }
}
