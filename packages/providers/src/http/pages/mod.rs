//! Page image downloads on the same footing as API calls: the host's shared token bucket
//! ([`HostLimiters::page_bucket`]), [`RetryPolicy`] with `Retry-After`, transient-error
//! classification, the source's page headers ([`super::HostPolicy`]) and public hosts only.

mod attempt;
mod redirect;

use std::time::Duration;

use async_trait::async_trait;
use mangas_core::domain::{Error, Result};
use mangas_core::download::{is_public_http_url, PageFetcher};
use mangas_core::sources::{Header, Source};
use reqwest::Client;

use super::{header_map, RetryPolicy};
use crate::rate_limit::HostLimiters;

/// Page requests per second to an image host that no provider declared a rate for.
pub const DEFAULT_PAGE_RATE: u32 = 4;

/// Page retries: more patient than API calls. A CDN in front of a quota-limited origin (Comick)
/// answers 429 to about half of the cache misses even one at a time, in windows of a few
/// seconds, so a page needs several tries (1 s, 2 s, 4 s, ... up to 30 s).
pub const PAGE_RETRY: RetryPolicy = RetryPolicy {
    max_retries: 10,
    base_delay: Duration::from_secs(1),
    max_delay: Duration::from_secs(30),
};

/// The final response to a page request, once retries are over.
#[derive(Debug)]
pub struct Fetched {
    pub status: u16,
    pub body: Vec<u8>,
    pub attempts: u32,
}

/// Fetches the page images of one [`Source`]; the downloader's [`PageFetcher`].
pub struct PageClient<'a> {
    source: &'a dyn Source,
    http: Client,
    retry: RetryPolicy,
    public_only: bool,
}

impl<'a> PageClient<'a> {
    pub fn for_source(source: &'a dyn Source) -> Result<Self> {
        Ok(Self {
            source,
            http: redirect::build_http(true)?,
            retry: PAGE_RETRY,
            public_only: true,
        })
    }

    pub fn with_retry(self, retry: RetryPolicy) -> Self {
        Self { retry, ..self }
    }

    /// Also reaches loopback and private addresses: only for tests against a mock server.
    pub fn allow_private_hosts(self) -> Result<Self> {
        Ok(Self {
            http: redirect::build_http(false)?,
            public_only: false,
            ..self
        })
    }

    /// One-byte ranged GET of `url` (`providers check`); hosts that ignore `Range` answer 200.
    pub async fn first_byte(&self, url: &str) -> Result<u16> {
        let range = ("range".to_string(), "bytes=0-0".to_string());
        Ok(self.get(url, &[range]).await?.status)
    }

    /// GET `url` with the source's page headers plus `extra`, paced by the host's bucket and
    /// retried on transient failures. A 429 pauses the whole host, not just this request. Any
    /// final status is returned for the caller to judge.
    pub async fn get(&self, url: &str, extra: &[Header]) -> Result<Fetched> {
        if self.public_only && !is_public_http_url(url) {
            return Err(self.error(format!("refusing to fetch non-public url {url}")));
        }
        let mut headers = self.source.page_headers(url);
        headers.extend_from_slice(extra);
        let headers = header_map(&headers)?;
        let bucket = HostLimiters::shared().page_bucket(url, DEFAULT_PAGE_RATE);
        let mut attempt = 0;
        loop {
            bucket.acquire().await;
            let request = self.http.get(url).headers(headers.clone());
            match self.attempt(url, request, attempt).await {
                (Ok(Fetched { status: 429, .. }), Some(wait)) => bucket.pause(wait).await,
                (_, Some(wait)) => tokio::time::sleep(wait).await,
                (result, None) => return result,
            }
            attempt += 1;
        }
    }

    fn error(&self, message: String) -> Error {
        Error::Source {
            name: self.source.name().to_string(),
            message,
        }
    }
}

#[async_trait]
impl PageFetcher for PageClient<'_> {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        let fetched = self.get(url, &[]).await?;
        if (200..300).contains(&fetched.status) {
            return Ok(fetched.body);
        }
        let tries = attempt::attempts(fetched.attempts);
        Err(self.error(format!("HTTP {} for {url} ({tries})", fetched.status)))
    }
}
