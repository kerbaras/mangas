mod transport;

use std::sync::Arc;

use chrono::Utc;
use mangas_core::domain::{Error, Result};
use mangas_core::sources::{Header, ProviderStatus};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::{
    header_map, probe_status, redact_query, session_gate, ClientConfig, HostPolicy, Reply,
    RetryPolicy, USER_AGENT,
};
use crate::rate_limit::{HostLimiters, TokenBucket};
use crate::session::Session;

/// HTTP client for one site: the host's shared token bucket ([`HostLimiters`]), timeouts, retries with backoff, browser
/// session headers, challenge detection and the page-header policy.
pub struct SiteClient {
    config: ClientConfig,
    http: Client,
    bucket: Arc<TokenBucket>,
    policy: HostPolicy,
}

impl SiteClient {
    pub fn new(config: ClientConfig, session: Option<Session>) -> Result<Self> {
        let headers = session.as_ref().map(Session::headers).unwrap_or_default();
        let http = Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(header_map(&headers)?)
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .build()?;
        let limiters = HostLimiters::shared();
        let bucket = limiters.bucket(&config.base, config.requests_per_second);
        limiters.declare_image_hosts(config.image_hosts, config.requests_per_second);
        Ok(Self {
            policy: HostPolicy::new(config.info.site, config.image_hosts, session),
            bucket,
            http,
            config,
        })
    }

    /// Headers for downloading the page image at `url` (see [`HostPolicy`]).
    pub fn page_headers(&self, url: &str) -> Vec<Header> {
        self.policy.headers_for(url)
    }

    /// GET `base + path`; a verification page becomes [`Error::Challenge`], any other final
    /// response is returned for the provider to interpret.
    pub async fn get_raw(&self, path: &str, query: &[(&str, String)]) -> Result<Reply> {
        let url = self.url(path);
        let id = self.config.info.id;
        log::debug!("{id}: GET {url} {}", redact_query(query));
        log::trace!("{id}: GET {url} {query:?}");
        let retry = self.config.retry;
        self.send(path, || self.http.get(&url).query(query), true, retry)
            .await
    }

    /// GET returning the body of a 2xx response; other statuses become [`Error::Source`].
    pub async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<String> {
        let reply = self.get_raw(path, query).await?;
        self.ok_body(path, reply)
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let body = self.get(path, query).await?;
        self.decode(path, &body)
    }

    /// POST a JSON body and decode a JSON reply. Retried only when the server did not act.
    pub async fn post_json<B, T>(&self, path: &str, body: &B) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = self.url(path);
        log::debug!("{}: POST {url}", self.config.info.id);
        let retry = self.config.retry;
        let reply = self
            .send(path, || self.http.post(&url).json(body), false, retry)
            .await?;
        let text = self.ok_body(path, reply)?;
        self.decode(path, &text)
    }

    /// One GET of `probe` (no retries) unless the session state already decides it.
    pub async fn status(&self, probe: &str) -> ProviderStatus {
        let info = &self.config.info;
        let session = self.policy.session();
        let ttl = self.config.session_ttl;
        if let Some(status) = session_gate(info.needs_browser_session, session, ttl, Utc::now()) {
            return status;
        }
        let url = self.url(probe);
        let reply = self
            .send(probe, || self.http.get(&url), true, RetryPolicy::NONE)
            .await;
        probe_status(&reply.and_then(|r| self.ok_body(probe, r)))
    }

    fn challenge(&self) -> Error {
        Error::Challenge {
            name: self.config.info.id.to_string(),
            site: self.config.info.site.to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.config.base)
    }

    fn ok_body(&self, path: &str, reply: Reply) -> Result<String> {
        if (200..300).contains(&reply.status) {
            Ok(reply.body)
        } else {
            Err(self.error(format!("HTTP {} for {path}", reply.status)))
        }
    }

    fn decode<T: DeserializeOwned>(&self, path: &str, body: &str) -> Result<T> {
        serde_json::from_str(body).map_err(|e| self.error(format!("{path}: unexpected JSON: {e}")))
    }

    fn error(&self, message: String) -> Error {
        Error::Source {
            name: self.config.info.id.to_string(),
            message,
        }
    }
}
