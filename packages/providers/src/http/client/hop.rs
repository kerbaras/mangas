//! One request of the [`SiteClient`] redirect chain: what is sent, with which headers, and
//! which host's token bucket paces it.

use std::sync::Arc;

use mangas_core::domain::Result;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest::{Method, RequestBuilder, Url};

use super::super::header_map;
use super::super::redirect::{same_origin, Hop};
use super::SiteClient;
use crate::rate_limit::{host_key, HostLimiters, TokenBucket};

/// A request about to be sent: the first one, or a hop of its redirect chain.
#[derive(Debug, Clone)]
pub(in crate::http) struct Outgoing {
    pub method: Method,
    pub url: Url,
    pub json: Option<Vec<u8>>,
    /// The reply body is read as bytes ([`super::super::Reply::raw`]), not text.
    pub binary: bool,
}

impl Outgoing {
    pub fn new(method: Method, url: Url, json: Option<Vec<u8>>) -> Self {
        Self {
            method,
            url,
            json,
            binary: false,
        }
    }

    /// The request a redirect asks for; the body only goes along on a 307/308.
    pub fn follow(self, hop: Hop) -> Self {
        let json = self.json.filter(|_| hop.keep_body);
        Self {
            binary: self.binary,
            ..Self::new(hop.method, hop.url, json)
        }
    }

    /// A POST (or PATCH) may have been acted on, so it is not repeated blindly.
    pub fn idempotent(&self) -> bool {
        self.method != Method::POST && self.method != Method::PATCH
    }
}

impl SiteClient {
    /// `base + path` with `query` appended (no `?` when empty).
    pub(super) fn parse(&self, url: &str, query: &[(&str, String)]) -> Result<Url> {
        let mut parsed = Url::parse(url).map_err(|e| self.error(format!("{url}: {e}")))?;
        if !query.is_empty() {
            parsed
                .query_pairs_mut()
                .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        }
        Ok(parsed)
    }

    /// The site's request headers and browser session while the chain stays on the first
    /// request's origin; anywhere else, only what [`super::HostPolicy`] grants that host (the
    /// referer for declared image hosts, the session within the site).
    pub(super) fn hop_headers(&self, url: &Url, first: &Url) -> Result<HeaderMap> {
        if !same_origin(url, first) {
            return header_map(&self.policy.headers_for(url.as_str()));
        }
        let mut headers = self.policy.request_headers(url.as_str());
        headers.extend(self.config.headers.iter().cloned());
        if let Some(session) = self.policy.session() {
            headers.extend(session.headers());
        }
        header_map(&headers)
    }

    /// The client's own bucket on its base host; another host's shared bucket (created with
    /// this client's rate if no one paced that host yet) for a hop that left it.
    pub(super) fn hop_bucket(&self, url: &Url) -> Arc<TokenBucket> {
        if host_key(url.as_str()) == host_key(&self.config.base) {
            return Arc::clone(&self.bucket);
        }
        HostLimiters::shared().page_bucket(url.as_str(), self.config.requests_per_second)
    }

    pub(super) fn build(&self, request: &Outgoing, headers: HeaderMap) -> RequestBuilder {
        let builder = self
            .http
            .request(request.method.clone(), request.url.clone())
            .headers(headers);
        match &request.json {
            Some(json) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(json.clone()),
            None => builder,
        }
    }
}
