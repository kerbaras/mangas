use std::net::IpAddr;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::{Attempt, Policy};
use reqwest::Client;
use url::{Host, Url};

use crate::domain::{Error, Result};
use crate::sources::{Header, Source};

/// Fetches one page image. Abstracted so downloads can be tested without a network.
#[async_trait]
pub trait PageFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>>;
}

pub struct HttpFetcher {
    http: Client,
}

impl HttpFetcher {
    pub fn new() -> Result<Self> {
        Self::with_headers(&[])
    }

    /// Fetcher sending the headers `source` needs on every page request.
    pub fn for_source(source: &dyn Source) -> Result<Self> {
        Self::with_headers(&source.page_headers())
    }

    pub fn with_headers(headers: &[Header]) -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!("mangas/", env!("CARGO_PKG_VERSION")))
            .default_headers(header_map(headers)?)
            .redirect(Policy::custom(public_redirects_only))
            .build()?;
        Ok(Self { http })
    }
}

fn header_map(headers: &[Header]) -> Result<HeaderMap> {
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

#[async_trait]
impl PageFetcher for HttpFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        if !is_public_http_url(url) {
            return Err(Error::Other(format!(
                "refusing to fetch non-public url {url}"
            )));
        }
        let response = self.http.get(url).send().await?.error_for_status()?;
        if !is_public_http_url(response.url().as_str()) {
            return Err(Error::Other(format!(
                "refusing response from non-public url {}",
                response.url()
            )));
        }
        Ok(response.bytes().await?.to_vec())
    }
}

const MAX_REDIRECTS: usize = 10;

/// Redirects get the same host check as the first request, so a public page URL cannot bounce
/// the downloader onto an internal address.
fn public_redirects_only(attempt: Attempt) -> reqwest::redirect::Action {
    if attempt.previous().len() >= MAX_REDIRECTS {
        attempt.error("too many redirects")
    } else if is_public_http_url(attempt.url().as_str()) {
        attempt.follow()
    } else {
        attempt.error("redirect to a non-public url")
    }
}

/// True for `http(s)` URLs whose host is a public name or address. Page URLs come from the
/// source's response, so this keeps a hostile response from pointing the downloader at
/// localhost or the LAN.
pub fn is_public_http_url(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host() {
        Some(Host::Domain(d)) => !d.eq_ignore_ascii_case("localhost") && !d.ends_with(".localhost"),
        Some(Host::Ipv4(ip)) => is_public_ip(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => is_public_ip(IpAddr::V6(ip)),
        None => false,
    }
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, _, _] = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || a == 0
                || a >= 240
                || (a == 100 && (64..128).contains(&b))
                || (a == 198 && (18..20).contains(&b))
                || (a == 192 && b == 0 && v4.octets()[2] == 0))
        }
        IpAddr::V6(v6) => {
            let seg = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || (seg & 0xfe00) == 0xfc00
                || (seg & 0xffc0) == 0xfe80
                || v6
                    .to_ipv4_mapped()
                    .is_some_and(|v4| !is_public_ip(IpAddr::V4(v4))))
        }
    }
}
