//! How provider requests reach the network ([`Network`], from the settings), applied once to
//! each reqwest client builder. The CLI's `--proxy` flag overrides it for the whole process.

use std::sync::OnceLock;

use mangas_core::domain::Error;
use mangas_core::library::Network;
use reqwest::{ClientBuilder, Proxy, Url};

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
pub(crate) fn apply(builder: ClientBuilder, network: &Network) -> Result<ClientBuilder, Error> {
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
pub(crate) fn proxied(network: &Network) -> bool {
    match effective(network) {
        Network::System => env_proxied(),
        Network::Direct => false,
        Network::Proxy(_) => true,
    }
}

/// Whether reqwest picks up a proxy from the environment (it reads these variables).
fn env_proxied() -> bool {
    [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]
    .iter()
    .any(|var| std::env::var_os(var).is_some_and(|v| !v.is_empty()))
}

/// A transport error, naming the proxy when a configured one could not be reached (a
/// connection failure then means the proxy, not the site, is down or refusing).
pub(crate) fn transport_error(e: reqwest::Error, network: &Network) -> Error {
    match effective(network) {
        Network::Proxy(url) if e.is_connect() => Error::Other(format!(
            "network: couldn't connect through the proxy {}: {e}",
            shown(&url)
        )),
        _ => e.into(),
    }
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
}
