//! DNS for page and cover downloads: a name only connects if every address it resolves to is
//! public. Checking the resolved addresses (not the host text) stops a public-looking name that
//! points at 127.0.0.1, the LAN or the cloud metadata address, and since the checked addresses
//! are the ones the connection uses, a second lookup cannot swap them.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use mangas_core::domain::Error;
use mangas_core::download::{is_public_ip, unmapped};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::{Host, Url};

/// Resolves with the system, or answers every name with `fixed` (tests only).
#[derive(Default)]
pub(super) struct PublicResolver {
    pub(super) fixed: Option<IpAddr>,
}

impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let fixed = self.fixed;
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = match fixed {
                Some(ip) => vec![SocketAddr::new(ip, 0)],
                None => tokio::net::lookup_host((host.as_str(), 0)).await?.collect(),
            };
            let addrs = public_only(&host, addrs)?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// Why a host is not fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    NonPublic { host: String, ip: IpAddr },
    NoAddress(String),
    Unresolved { host: String, reason: String },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPublic { host, ip } => {
                write!(f, "{host} resolves to a non-public address {ip}")
            }
            Self::NoAddress(host) => write!(f, "{host} resolves to no address"),
            Self::Unresolved { host, reason } => write!(f, "{host}: {reason}"),
        }
    }
}

impl std::error::Error for Refusal {}

impl Refusal {
    /// The app error: [`Error::BlockedAddress`] for a private address (final, never retried).
    pub(super) fn into_error(self, source: &str) -> Error {
        match self {
            Self::NonPublic { host, ip } => Error::BlockedAddress { host, ip },
            other => Error::Source {
                name: source.to_string(),
                message: other.to_string(),
            },
        }
    }
}

/// The [`Refusal`] behind a failed request: reqwest wraps the resolver's error as a connect
/// error, which would otherwise be retried as network trouble.
pub(super) fn refusal_in(error: &(dyn std::error::Error + 'static)) -> Option<Refusal> {
    let mut current = Some(error);
    while let Some(e) = current {
        if let Some(refusal) = e.downcast_ref::<Refusal>() {
            return Some(refusal.clone());
        }
        current = e.source();
    }
    None
}

/// Checks `host` up front when requests go through a proxy: the connection then resolves the
/// proxy, not the host, so [`PublicResolver`] never sees it.
pub(super) async fn check_proxied(host: &str) -> Result<(), Refusal> {
    let addrs = tokio::net::lookup_host((host, 0))
        .await
        .map_err(|e| Refusal::Unresolved {
            host: host.to_string(),
            reason: e.to_string(),
        })?
        .collect();
    public_only(host, addrs).map(|_| ())
}

/// The refusal for a URL whose host text is already non-public: a private IP literal or a
/// `localhost` name, which RFC 6761 pins to loopback. Other hosts wait for DNS.
pub(super) fn literal_refusal(raw: &str) -> Option<Refusal> {
    let url = Url::parse(raw).ok()?;
    let ip = match url.host()? {
        Host::Ipv4(v4) => IpAddr::V4(v4),
        Host::Ipv6(v6) => unmapped(IpAddr::V6(v6)),
        Host::Domain(d) if d == "localhost" || d.ends_with(".localhost") => {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        }
        Host::Domain(_) => return None,
    };
    let host = match url.host()? {
        Host::Ipv6(v6) if v6.to_ipv4_mapped().is_some() => ip.to_string(),
        _ => url.host_str()?.to_string(),
    };
    (!is_public_ip(ip)).then_some(Refusal::NonPublic { host, ip })
}

/// `addrs` when all of them are public; a refusal naming the first private one otherwise. One
/// private address is enough to refuse: the connector may pick any of them.
pub fn public_only(host: &str, addrs: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, Refusal> {
    match addrs.iter().find(|a| !is_public_ip(a.ip())) {
        Some(private) => Err(Refusal::NonPublic {
            host: host.to_string(),
            ip: unmapped(private.ip()),
        }),
        None if addrs.is_empty() => Err(Refusal::NoAddress(host.to_string())),
        None => Ok(addrs),
    }
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod tests;
