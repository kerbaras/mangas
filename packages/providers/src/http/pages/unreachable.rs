//! Page hosts that can't be looked up or reached (#311): directly, when the resolver's lookup
//! fails, or through the proxy, when it answers with its own error instead of the site's. A name
//! that doesn't resolve now won't in the next seconds, so these get one retry, not ten.

use mangas_core::domain::Error;
use mangas_core::library::Network;
use reqwest::Response;
use url::Url;

use super::super::network::{effective, shown};

/// Retries for a host that can't be looked up or that the proxy can't reach.
pub(super) const UNREACHABLE_RETRIES: u32 = 1;

/// Whether `e` is the proxy refusing the `CONNECT` tunnel (any answer but 200 or 407).
pub(super) fn tunnel_refused(e: &(dyn std::error::Error + 'static)) -> bool {
    let mut current = Some(e);
    while let Some(e) = current {
        if e.to_string() == "tunnel error: unsuccessful" {
            return true;
        }
        current = e.source();
    }
    false
}

/// Whether a plain-http `response` through a proxy is the proxy's own 5xx rather than the
/// site's: forwarded responses carry `Via`, and Squid names its own errors in `X-Squid-Error`.
/// An https page's response always comes from the site, inside the tunnel.
pub(super) fn proxy_answered(url: &str, response: &Response) -> bool {
    let headers = response.headers();
    url.starts_with("http:")
        && response.status().is_server_error()
        && (headers.contains_key("x-squid-error") || !headers.contains_key("via"))
}

/// What the proxy said: the status, and Squid's error name when it gives one (`ERR_DNS_FAIL`).
pub(super) fn proxy_answer(response: &Response) -> String {
    let squid = response
        .headers()
        .get("x-squid-error")
        .and_then(|v| v.to_str().ok());
    match squid {
        Some(error) => format!("HTTP {}, {error}", response.status().as_u16()),
        None => format!("HTTP {}", response.status().as_u16()),
    }
}

/// The error for a page on `url` that the proxy of `network` couldn't reach.
pub(super) fn through_proxy(network: &Network, url: &str, what: &str) -> Error {
    let proxy = match effective(network) {
        Network::Proxy(proxy) => format!("the proxy {}", shown(&proxy)),
        _ => "the system proxy".to_string(),
    };
    let host = Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| url.to_string());
    Error::Other(format!(
        "network: {proxy} couldn't reach {host} ({what}); it may not resolve that name"
    ))
}
