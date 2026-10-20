//! Provider clients reach the network through [`mangas_core::net`].

use hyper_util::client::proxy::matcher::Matcher;
use mangas_core::library::Network;

pub(crate) use mangas_core::net::{apply, transport_error};
pub use mangas_core::net::{effective, override_network, shown};

/// The host names of the proxies provider clients go through, lowercased: the configured one,
/// or in system mode the environment's and the OS settings' (macOS, Windows), read the way
/// reqwest's `system-proxy` reads them.
pub(crate) fn proxy_hosts(network: &Network) -> Vec<String> {
    let mut hosts = mangas_core::net::proxy_hosts(network);
    if matches!(effective(network), Network::System) {
        let system = Matcher::from_system();
        let proxy_of = |dst: &str| {
            let dst = dst.parse::<http::Uri>().ok()?;
            Some(system.intercept(&dst)?.uri().host()?.to_ascii_lowercase())
        };
        hosts.extend(
            ["http://mangas.invalid/", "https://mangas.invalid/"]
                .map(proxy_of)
                .into_iter()
                .flatten(),
        );
    }
    hosts.sort();
    hosts.dedup();
    hosts
}
