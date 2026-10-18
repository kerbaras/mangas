//! Cover fetching with the provider's page policy: the same headers as page downloads
//! (`Source::page_headers`, i.e. the provider's [`crate::http::HostPolicy`]), the shared
//! per-host rate limit and the client retry policy. Plugs into `mangas_core::covers::cover`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use mangas_core::domain::{Error, Result};
use mangas_core::download::{is_public_http_url, PageFetcher};
use reqwest::header::RETRY_AFTER;
use reqwest::redirect::Policy;
use reqwest::{Client, Response};

use crate::http::{header_map, parse_retry_after, retryable_status, RetryPolicy, USER_AGENT};
use crate::rate_limit::HostLimiters;
use crate::Source;

/// Cover CDNs get the default client budget unless a provider declared a stricter one for the
/// same host.
const REQUESTS_PER_SECOND: u32 = 3;
/// Larger bodies are not covers.
const MAX_BYTES: usize = 20 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;
/// A stalled CDN fails the cover (and frees its placeholder for a retry) instead of hanging.
const TIMEOUT: Duration = Duration::from_secs(30);

pub struct CoverFetcher {
    source: Arc<dyn Source>,
    http: Client,
    retry: RetryPolicy,
    public_only: bool,
}

impl CoverFetcher {
    /// Fetches covers for `source`, refusing hosts that are not public (the URL comes from a
    /// provider response).
    pub fn for_source(source: Arc<dyn Source>) -> Result<Self> {
        Self::build(source, RetryPolicy::default(), true)
    }

    /// Like [`CoverFetcher::for_source`] with any host allowed and the given retries (tests
    /// against a local mock server).
    pub fn unguarded(source: Arc<dyn Source>, retry: RetryPolicy) -> Result<Self> {
        Self::build(source, retry, false)
    }

    fn build(source: Arc<dyn Source>, retry: RetryPolicy, public_only: bool) -> Result<Self> {
        let http = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(TIMEOUT)
            .redirect(Policy::custom(move |attempt| {
                if attempt.previous().len() >= MAX_REDIRECTS {
                    attempt.error("too many redirects")
                } else if public_only && !is_public_http_url(attempt.url().as_str()) {
                    attempt.error("redirect to a non-public url")
                } else {
                    attempt.follow()
                }
            }))
            .build()?;
        Ok(Self {
            source,
            http,
            retry,
            public_only,
        })
    }

    async fn send(&self, url: &str) -> Result<Response> {
        let headers = header_map(&self.source.page_headers(url))?;
        let bucket = HostLimiters::shared().bucket(url, REQUESTS_PER_SECOND);
        let mut attempt = 0;
        loop {
            bucket.acquire().await;
            let response = self.http.get(url).headers(headers.clone()).send().await?;
            let status = response.status().as_u16();
            if !retryable_status(status, true) {
                return Ok(response);
            }
            let after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| parse_retry_after(v, Utc::now()));
            match self.retry.delay(attempt, after) {
                Some(wait) => tokio::time::sleep(wait).await,
                None => return Ok(response),
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
impl PageFetcher for CoverFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        if self.public_only && !is_public_http_url(url) {
            return Err(self.error(format!("refusing cover from non-public url {url}")));
        }
        let response = self.send(url).await?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(self.error(format!("HTTP {status} for cover {url}")));
        }
        let too_big = || self.error(format!("cover {url} is over {MAX_BYTES} bytes"));
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BYTES as u64)
        {
            return Err(too_big());
        }
        let mut response = response;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len() + chunk.len() > MAX_BYTES {
                return Err(too_big());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}
