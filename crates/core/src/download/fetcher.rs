use std::net::IpAddr;

use async_trait::async_trait;
use reqwest::redirect::{Attempt, Policy};
use reqwest::Client;
use url::{Host, Url};

use crate::domain::{Error, Result};

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
        let http = Client::builder()
            .user_agent(concat!("mangas/", env!("CARGO_PKG_VERSION")))
            .redirect(Policy::custom(public_redirects_only))
            .build()?;
        Ok(Self { http })
    }
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
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
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
