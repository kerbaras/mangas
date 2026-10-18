use std::net::IpAddr;

use async_trait::async_trait;
use url::{Host, Url};

use crate::domain::Result;

/// Fetches one page image. The apps use the providers' `http::PageClient` (the host's rate
/// limit, retries, page headers); tests inject fakes.
#[async_trait]
pub trait PageFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>>;
}

/// A borrowed fetcher fetches like the one it points to.
#[async_trait]
impl<T: PageFetcher + ?Sized> PageFetcher for &T {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        (**self).fetch(url).await
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
