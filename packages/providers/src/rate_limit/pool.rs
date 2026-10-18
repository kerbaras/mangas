use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use super::shared::file_name;
use super::TokenBucket;
use crate::http::host_matches;

/// Token buckets keyed by host. Clients ask for their host's bucket instead of owning one, so
/// two clients (or two sources built by separate calls) for the same host share one budget.
/// When clients declare different rates for one host, the strictest one applies.
#[derive(Default)]
pub struct HostLimiters {
    buckets: Mutex<HashMap<String, Arc<TokenBucket>>>,
    /// Image host patterns (see [`host_matches`]) and the rate their provider declared.
    declared: Mutex<Vec<(String, u32)>>,
    /// Folder of the per-host bucket files once the buckets are shared across processes.
    dir: Mutex<Option<PathBuf>>,
}

impl HostLimiters {
    /// The process-wide pool every [`crate::http::SiteClient`] uses.
    pub fn shared() -> &'static HostLimiters {
        static SHARED: OnceLock<HostLimiters> = OnceLock::new();
        SHARED.get_or_init(HostLimiters::default)
    }

    /// Keeps every bucket (existing and future) in a file under `dir`, one per host, so every
    /// process pointing at the same folder shares one budget per host. The apps pass a folder
    /// in the platform data dir, which the CLI and the desktop app have in common.
    pub fn share_across_processes(&self, dir: PathBuf) {
        let buckets = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);
        for (key, bucket) in buckets.iter() {
            bucket.share_in(dir.join(file_name(key)));
        }
        *self.dir.lock().unwrap_or_else(PoisonError::into_inner) = Some(dir);
    }

    /// The bucket for the host of `url`, created on first use with a burst of
    /// `requests_per_second` and tightened if a later caller asks for less.
    pub fn bucket(&self, url: &str, requests_per_second: u32) -> Arc<TokenBucket> {
        let rate = requests_per_second.max(1);
        let key = host_key(url);
        let mut buckets = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);
        let bucket = buckets.entry(key.clone()).or_insert_with(|| {
            let bucket = TokenBucket::new(rate, rate);
            let dir = self.dir.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(dir) = dir.as_ref() {
                bucket.share_in(dir.join(file_name(&key)));
            }
            Arc::new(bucket)
        });
        bucket.tighten(rate, rate);
        Arc::clone(bucket)
    }

    /// Records that page images on hosts matching `patterns` (exact host or a subdomain) should
    /// go no faster than `requests_per_second`. `"*"` is ignored: one provider's rate must not
    /// apply to every other site's images.
    pub fn declare_image_hosts(&self, patterns: &[&str], requests_per_second: u32) {
        let mut declared = self.declared.lock().unwrap_or_else(PoisonError::into_inner);
        let rate = requests_per_second.max(1);
        for pattern in patterns.iter().filter(|p| **p != "*" && !p.is_empty()) {
            let entry = (pattern.to_ascii_lowercase(), rate);
            if !declared.contains(&entry) {
                declared.push(entry);
            }
        }
    }

    /// The bucket a page image download of `url` draws from: the host's existing bucket (its
    /// API client's budget, if the site serves its own images), tightened to a rate declared
    /// for that image host; a new host gets the declared rate or `default_rate`.
    pub fn page_bucket(&self, url: &str, default_rate: u32) -> Arc<TokenBucket> {
        let declared = self.declared_rate(url);
        let existing = self
            .buckets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&host_key(url))
            .cloned();
        match (existing, declared) {
            (Some(bucket), None) => bucket,
            (_, rate) => self.bucket(url, rate.unwrap_or(default_rate)),
        }
    }

    fn declared_rate(&self, url: &str) -> Option<u32> {
        let host = url::Url::parse(url).ok()?.host_str()?.to_string();
        let declared = self.declared.lock().unwrap_or_else(PoisonError::into_inner);
        declared
            .iter()
            .filter(|(pattern, _)| host_matches(&host, pattern))
            .map(|(_, rate)| *rate)
            .min()
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
