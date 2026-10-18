//! The page client's HTTP client: timeouts, user agent, no automatic redirects (the client
//! follows them hop by hop, see [`super::PageClient::get`]) and, for real downloads, DNS that
//! only connects to public addresses.

#[cfg(any(test, feature = "test-support"))]
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use mangas_core::domain::Result;
use reqwest::redirect::Policy;
use reqwest::{Client, ClientBuilder};

use super::super::USER_AGENT;
use super::resolver::PublicResolver;

const TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) fn build_http(public_only: bool) -> Result<Client> {
    let builder = if public_only {
        builder().dns_resolver(Arc::new(PublicResolver::default()))
    } else {
        builder()
    };
    Ok(builder.build()?)
}

/// A public-only client whose DNS answers every name with `ip` (tests only).
#[cfg(any(test, feature = "test-support"))]
pub(super) fn build_http_resolving(ip: IpAddr) -> Result<Client> {
    let resolver = PublicResolver { fixed: Some(ip) };
    Ok(builder().dns_resolver(Arc::new(resolver)).build()?)
}

fn builder() -> ClientBuilder {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(Policy::none())
}
