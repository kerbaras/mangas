//! DNS for page and cover downloads: a name only connects if every address it resolves to is
//! public. Checking the resolved addresses (not the host text) stops a public-looking name that
//! points at 127.0.0.1, the LAN or the cloud metadata address, and since the checked addresses
//! are the ones the connection uses, a second lookup cannot swap them.

use std::net::SocketAddr;

use mangas_core::download::is_public_ip;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

pub(super) struct PublicResolver;

impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            let addrs = public_only(&host, addrs)?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// Checks `host` up front when requests go through a proxy: the connection then resolves the
/// proxy, not the host, so [`PublicResolver`] never sees it.
pub(super) async fn check_proxied(host: &str) -> Result<(), String> {
    if !proxied() {
        return Ok(());
    }
    let addrs = tokio::net::lookup_host((host, 0))
        .await
        .map_err(|e| format!("{host}: {e}"))?
        .collect();
    public_only(host, addrs).map(|_| ())
}

/// Whether reqwest picks up a proxy from the environment (it reads these variables).
fn proxied() -> bool {
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

/// `addrs` when all of them are public; an error naming the first private one otherwise. One
/// private address is enough to refuse: the connector may pick any of them.
pub fn public_only(host: &str, addrs: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, String> {
    match addrs.iter().find(|a| !is_public_ip(a.ip())) {
        Some(private) => Err(format!(
            "{host} resolves to non-public address {}",
            private.ip()
        )),
        None if addrs.is_empty() => Err(format!("{host} resolves to no address")),
        None => Ok(addrs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(ip: &str) -> SocketAddr {
        SocketAddr::new(ip.parse().unwrap(), 0)
    }

    #[test]
    fn public_addresses_pass() {
        let addrs = vec![addr("93.184.216.34"), addr("2606:2800:220:1::")];
        assert_eq!(public_only("cdn", addrs.clone()), Ok(addrs));
    }

    #[test]
    fn any_private_address_refuses_the_name() {
        for bad in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.0.9",
            "169.254.169.254",
            "::1",
            "fd00::1",
        ] {
            let addrs = vec![addr("93.184.216.34"), addr(bad)];
            let err = public_only("evil.example", addrs).unwrap_err();
            assert!(err.contains(bad), "{err}");
        }
    }

    #[test]
    fn no_address_is_refused() {
        assert!(public_only("void", vec![]).is_err());
    }
}
