use chrono::Utc;
use mangas_core::domain::{Error, Result};
use mangas_core::sources::{Header, ProviderStatus};
use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::{
    header_map, is_challenge, parse_retry_after, probe_status, retryable_status, session_gate,
    ClientConfig, HostPolicy, Reply, RetryPolicy, USER_AGENT,
};
use crate::rate_limit::TokenBucket;
use crate::session::Session;

/// HTTP client for one site: per-client token bucket, timeouts, retries with backoff, browser
/// session headers, challenge detection and the page-header policy.
pub struct SiteClient {
    config: ClientConfig,
    http: Client,
    bucket: TokenBucket,
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
        let rate = config.requests_per_second.max(1);
        Ok(Self {
            policy: HostPolicy::new(config.info.site, config.image_hosts, session),
            bucket: TokenBucket::new(rate, rate),
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
        log::debug!("{}: GET {url} {query:?}", self.config.info.id);
        let retry = self.config.retry;
        self.send(|| self.http.get(&url).query(query), true, retry)
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
            .send(|| self.http.post(&url).json(body), false, retry)
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
            .send(|| self.http.get(&url), true, RetryPolicy::NONE)
            .await;
        probe_status(&reply.and_then(|r| self.ok_body(probe, r)))
    }

    async fn send(
        &self,
        request: impl Fn() -> RequestBuilder,
        idempotent: bool,
        retry: RetryPolicy,
    ) -> Result<Reply> {
        let mut attempt = 0;
        loop {
            self.bucket.acquire().await;
            let outcome = request().send().await;
            let asked = match &outcome {
                Ok(r) if should_retry(r, idempotent) => Some(retry_after(r)),
                Err(e) if retryable_transport(e, idempotent) => Some(None),
                _ => None,
            };
            match asked.and_then(|after| retry.delay(attempt, after)) {
                Some(wait) => {
                    log::debug!("{}: retry {} in {wait:?}", self.config.info.id, attempt + 1);
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                }
                None => return self.reply(outcome?).await,
            }
        }
    }

    async fn reply(&self, response: Response) -> Result<Reply> {
        let status = response.status().as_u16();
        let mitigated = mitigated(&response);
        let body = response.text().await?;
        if is_challenge(status, &mitigated, &body) {
            return Err(Error::Challenge {
                name: self.config.info.id.to_string(),
                site: self.config.info.site.to_string(),
            });
        }
        Ok(Reply { status, body })
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

fn mitigated(response: &Response) -> String {
    response
        .headers()
        .get("cf-mitigated")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

/// A challenge is never retried: only the user can pass it.
fn should_retry(response: &Response, idempotent: bool) -> bool {
    retryable_status(response.status().as_u16(), idempotent)
        && !mitigated(response).eq_ignore_ascii_case("challenge")
}

fn retry_after(response: &Response) -> Option<std::time::Duration> {
    let value = response.headers().get("retry-after")?.to_str().ok()?;
    parse_retry_after(value, Utc::now())
}

/// A failed connection never reached the server; a timeout or a dropped connection might have,
/// so only idempotent requests retry those.
fn retryable_transport(e: &reqwest::Error, idempotent: bool) -> bool {
    e.is_connect() || (idempotent && (e.is_timeout() || e.is_request()))
}
