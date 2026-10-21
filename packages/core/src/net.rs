//! How requests reach the network ([`Network`], from the settings), applied once to each
//! reqwest client builder: provider traffic and the detector downloads alike. The CLI's
//! `--proxy` flag overrides it for the whole process.

use std::sync::OnceLock;

use reqwest::{ClientBuilder, Proxy, Url};

use crate::domain::{certificate_problem, Error};
use crate::library::Network;

static OVERRIDE: OnceLock<Network> = OnceLock::new();

/// Uses `network` for every client this process builds, whatever the settings say. Only the
/// first call counts; returns whether this one did.
pub fn override_network(network: Network) -> bool {
    OVERRIDE.set(network).is_ok()
}

/// `network`, unless the process has an override.
pub fn effective(network: &Network) -> Network {
    OVERRIDE.get().unwrap_or(network).clone()
}

/// `builder` routed per `network`: the system proxy (reqwest's default), none, or a fixed one.
pub fn apply(builder: ClientBuilder, network: &Network) -> Result<ClientBuilder, Error> {
    Ok(match effective(network) {
        Network::System => builder,
        Network::Direct => builder.no_proxy(),
        Network::Proxy(url) => builder.proxy(Proxy::all(&url).map_err(|e| invalid(&url, &e))?),
    })
}

fn invalid(url: &str, e: &reqwest::Error) -> Error {
    Error::Other(format!("invalid proxy URL {}: {e}", shown(url)))
}

/// Whether requests go through a proxy, so the connection resolves the proxy, not the host.
pub fn proxied(network: &Network) -> bool {
    match effective(network) {
        Network::System => env_proxied(),
        Network::Direct => false,
        Network::Proxy(_) => true,
    }
}

/// The proxy variables reqwest reads from the environment.
const PROXY_VARS: [&str; 6] = [
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "ALL_PROXY",
    "all_proxy",
];

/// Whether reqwest picks up a proxy from the environment.
fn env_proxied() -> bool {
    PROXY_VARS
        .iter()
        .any(|var| std::env::var_os(var).is_some_and(|v| !v.is_empty()))
}

/// The host names of the proxies requests go through (the configured one, or the
/// environment's), lowercased: the only names a client resolves itself when proxied.
pub fn proxy_hosts(network: &Network) -> Vec<String> {
    let urls = match effective(network) {
        Network::System => PROXY_VARS
            .iter()
            .filter_map(|var| std::env::var(var).ok())
            .collect(),
        Network::Direct => vec![],
        Network::Proxy(url) => vec![url],
    };
    urls.iter().filter_map(|url| proxy_host(url)).collect()
}

/// The host of a proxy URL; a bare `host:port` (which reqwest takes as http) too.
fn proxy_host(raw: &str) -> Option<String> {
    let host = |s: &str| Some(Url::parse(s).ok()?.host_str()?.to_ascii_lowercase());
    host(raw).or_else(|| host(&format!("http://{raw}")))
}

/// A transport error, naming the proxy when a configured one could not be reached (a
/// connection failure then means the proxy, not the site, is down, refusing or silent). A
/// refused certificate is the site's own, answered through the proxy's tunnel.
pub fn transport_error(e: reqwest::Error, network: &Network) -> Error {
    if certificate_problem(&e).is_some() {
        return e.into();
    }
    match effective(network) {
        Network::Proxy(url) if e.is_connect() && e.is_timeout() => Error::Other(format!(
            "network: no answer through the proxy {}",
            shown(&url)
        )),
        Network::Proxy(url) if e.is_connect() => Error::Other(format!(
            "network: couldn't connect through the proxy {}: {e}",
            shown(&url)
        )),
        _ => e.into(),
    }
}

/// Why a request that never answered failed, naming the proxy it went through, if any.
pub fn no_answer(network: &Network, secs: u64) -> String {
    match effective(network) {
        Network::Proxy(url) => format!(
            "no answer within {secs} s through the proxy {}",
            shown(&url)
        ),
        _ => format!("no answer within {secs} s"),
    }
}

/// A client routed per `network`, for one-off downloads outside the provider clients.
pub fn client(network: &Network) -> Result<reqwest::Client, Error> {
    Ok(apply(reqwest::Client::builder(), network)?.build()?)
}

/// `url` without its password, for messages and logs.
pub fn shown(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut parsed) if parsed.password().is_some() => {
            let _ = parsed.set_password(Some("***"));
            parsed.to_string()
        }
        _ => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_the_password() {
        assert_eq!(
            shown("http://me:secret@proxy:8080"),
            "http://me:***@proxy:8080/"
        );
        assert_eq!(shown("http://proxy:8080"), "http://proxy:8080");
    }

    #[test]
    fn a_fixed_proxy_counts_as_proxied() {
        assert!(proxied(&Network::Proxy("http://127.0.0.1:1".into())));
        assert!(!proxied(&Network::Direct));
    }

    #[test]
    fn proxy_hosts_name_the_configured_proxy() {
        let proxy = |url: &str| proxy_hosts(&Network::Proxy(url.into()));
        assert_eq!(proxy("http://me:pw@Proxy.LAN:3128"), ["proxy.lan"]);
        assert_eq!(proxy("socks5h://localhost:1055"), ["localhost"]);
        assert_eq!(proxy("[::1]:8080"), ["[::1]"]);
        assert!(proxy_hosts(&Network::Direct).is_empty());
    }

    #[test]
    fn a_silent_request_names_the_proxy() {
        let proxy = Network::Proxy("http://me:pw@127.0.0.1:8888".into());
        assert_eq!(
            no_answer(&proxy, 15),
            "no answer within 15 s through the proxy http://me:***@127.0.0.1:8888/"
        );
        assert_eq!(no_answer(&Network::Direct, 15), "no answer within 15 s");
    }
}
