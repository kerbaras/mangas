//! Cover fetching on the page download path ([`PageClient`]): the source's page headers
//! ([`crate::http::HostPolicy`]), the shared per-host rate limit, retries, hop-by-hop redirects,
//! public hosts only (checked on the resolved addresses) and a body limit. Plugs into
//! `mangas_core::covers::cover`.

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::Result;
use mangas_core::download::PageFetcher;
use reqwest::Client;

use crate::http::{build_http, PageClient, RetryPolicy};
use crate::Source;

/// Larger bodies are not covers.
const MAX_BYTES: usize = 20 * 1024 * 1024;

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
        Ok(Self {
            http: build_http(public_only)?,
            source,
            retry,
            public_only,
        })
    }

    fn client(&self) -> PageClient<'_> {
        PageClient::with_http(self.source.as_ref(), self.http.clone(), self.public_only)
            .with_retry(self.retry)
            .with_max_bytes(MAX_BYTES)
    }
}

#[async_trait]
impl PageFetcher for CoverFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        self.client().fetch(url).await
    }
}
