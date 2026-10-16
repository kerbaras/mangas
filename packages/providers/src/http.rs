//! Rate-limited JSON/HTML client shared by every provider.

use mangas_core::domain::{Error, Result};
use mangas_core::sources::Header;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;

use crate::rate_limit::TokenBucket;
use crate::session::Session;

pub const USER_AGENT: &str = concat!(
    "mangas/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/kerbaras/mangas)"
);

/// Status and body of a response; providers map their own error payloads.
pub struct Reply {
    pub status: u16,
    pub body: String,
}

pub struct ApiClient {
    name: &'static str,
    site: String,
    base: String,
    http: Client,
    bucket: TokenBucket,
}

impl ApiClient {
    /// `site` is shown to the user when the provider asks for a browser session; `base` is the
    /// prefix of every path passed to [`ApiClient::get`]; `per_second` caps the request rate.
    pub fn new(name: &'static str, site: &str, base: &str, per_second: u32) -> Result<Self> {
        Self::with_session(name, site, base, per_second, None)
    }

    pub fn with_session(
        name: &'static str,
        site: &str,
        base: &str,
        per_second: u32,
        session: Option<Session>,
    ) -> Result<Self> {
        let headers = session.as_ref().map(Session::headers).unwrap_or_default();
        let http = Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(header_map(&headers)?)
            .build()?;
        Ok(Self {
            name,
            site: site.to_string(),
            base: base.to_string(),
            http,
            bucket: TokenBucket::new(per_second, per_second),
        })
    }

    /// GET `base + path`; a verification page becomes [`Error::Challenge`], anything else is
    /// returned as is for the provider to interpret.
    pub async fn get_raw(&self, path: &str, query: &[(&str, String)]) -> Result<Reply> {
        self.bucket.acquire().await;
        let url = format!("{}{}", self.base, path);
        log::debug!("{}: GET {url} {query:?}", self.name);
        let response = self.http.get(&url).query(query).send().await?;
        let status = response.status().as_u16();
        let mitigated = response
            .headers()
            .get("cf-mitigated")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = response.text().await?;
        if is_challenge(status, &mitigated, &body) {
            return Err(Error::Challenge {
                name: self.name.to_string(),
                site: self.site.clone(),
            });
        }
        Ok(Reply { status, body })
    }

    /// GET returning the body of a 2xx response; other statuses become [`Error::Source`].
    pub async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<String> {
        let reply = self.get_raw(path, query).await?;
        if (200..300).contains(&reply.status) {
            Ok(reply.body)
        } else {
            Err(Error::Source {
                name: self.name.to_string(),
                message: format!("HTTP {} for {path}", reply.status),
            })
        }
    }
}

/// Cloudflare marks challenge responses with `cf-mitigated: challenge`; older edges only send
/// the interstitial page with a 403.
pub fn is_challenge(status: u16, cf_mitigated: &str, body: &str) -> bool {
    cf_mitigated.eq_ignore_ascii_case("challenge")
        || (status == 403 && body.contains("challenge-platform"))
}

pub fn header_map(headers: &[Header]) -> Result<HeaderMap> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| Error::Other(format!("header {name}: {e}")))?;
        let value = HeaderValue::from_str(value)
            .map_err(|e| Error::Other(format!("header {name}: {e}")))?;
        map.insert(name, value);
    }
    Ok(map)
}
