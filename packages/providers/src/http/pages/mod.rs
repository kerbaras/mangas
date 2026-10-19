//! Page image downloads on the same footing as API calls: the host's shared token bucket
//! ([`HostLimiters::page_bucket`]), [`RetryPolicy`] with `Retry-After`, transient-error
//! classification, the source's page headers ([`super::HostPolicy`]) and public hosts only.
//! Covers go through the same client ([`crate::covers::CoverFetcher`]).

mod attempt;
mod builder;
mod redirect;
mod resolver;

use std::time::Duration;

use async_trait::async_trait;
use mangas_core::domain::{Error, Result};
use mangas_core::download::{is_public_http_url, PageFetcher};
use mangas_core::library::Network;
use mangas_core::sources::{Header, Source};
use reqwest::{Client, Url};

use super::redirect::MAX_REDIRECTS;
use super::{header_map, network, RetryPolicy};
use crate::rate_limit::{host_key, HostLimiters};

pub(crate) use redirect::build_http;
pub use resolver::{public_only, Refusal};

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
    max_bytes: Option<usize>,
    network: Network,
}

/// How one hop ended: the final response, or a redirect to follow.
enum Step {
    Done(Fetched),
    Redirect(Url),
}

impl<'a> PageClient<'a> {
    /// Through the system proxy; see [`Self::via`].
    pub fn for_source(source: &'a dyn Source) -> Result<Self> {
        Self::via(source, &Network::System)
    }

    /// Requests go out per `network` (the settings' proxy).
    pub fn via(source: &'a dyn Source, network: &Network) -> Result<Self> {
        let http = build_http(true, network)?;
        Ok(Self::with_http(source, http, true, network.clone()))
    }

    /// A client on an `http` built by [`build_http`] with the same `public_only` and `network`.
    pub(crate) fn with_http(
        source: &'a dyn Source,
        http: Client,
        public_only: bool,
        network: Network,
    ) -> Self {
        Self {
            source,
            http,
            retry: PAGE_RETRY,
            public_only,
            max_bytes: None,
            network,
        }
    }

    /// One-byte ranged GET of `url` (`providers check`); hosts that ignore `Range` answer 200.
    pub async fn first_byte(&self, url: &str) -> Result<u16> {
        let range = ("range".to_string(), "bytes=0-0".to_string());
        Ok(self.get(url, &[range]).await?.status)
    }

    /// GET `url` with the source's page headers plus `extra`, paced by the host's bucket and
    /// retried on transient failures. A 429 pauses the whole host, not just this request.
    /// Redirects are followed hop by hop: each hop gets its own host's bucket, headers and public
    /// check. Any final status is returned for the caller to judge.
    pub async fn get(&self, url: &str, extra: &[Header]) -> Result<Fetched> {
        let mut url = self.target(url)?;
        let first = host_key(&url);
        for _ in 0..=MAX_REDIRECTS {
            // Caller headers (a range, a credential) are for the first host only.
            let extra = if host_key(&url) == first { extra } else { &[] };
            match self.get_hop(&url, extra).await? {
                Step::Done(fetched) => return Ok(fetched),
                Step::Redirect(next) => url = self.target(next.as_str())?,
            }
        }
        Err(self.error(format!("more than {MAX_REDIRECTS} redirects for {url}")))
    }

    /// The URL to request for `url`: upgraded to https where the source knows the host serves
    /// it ([`Source::page_url`]), refused when its host is not public.
    fn target(&self, url: &str) -> Result<String> {
        let url = self.source.page_url(url);
        if self.public_only && !is_public_http_url(&url) {
            return Err(match resolver::literal_refusal(&url) {
                Some(refusal) => refusal.into_error(self.source.name()),
                None => self.error(format!("refusing to fetch non-public url {url}")),
            });
        }
        if url.starts_with("http:") {
            log::debug!("{}: fetching plain http {url}", self.source.name());
        }
        Ok(url)
    }

    async fn get_hop(&self, url: &str, extra: &[Header]) -> Result<Step> {
        let domain = Url::parse(url)
            .ok()
            .and_then(|u| u.domain().map(str::to_string));
        let proxied = self.public_only && network::proxied(&self.network);
        if let Some(host) = domain.filter(|_| proxied) {
            resolver::check_proxied(&host)
                .await
                .map_err(|e| e.into_error(self.source.name()))?;
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
                (Ok(Step::Done(Fetched { status: 429, .. })), Some(wait)) => {
                    bucket.pause(wait).await;
                }
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
