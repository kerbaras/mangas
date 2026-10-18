//! The page client's HTTP client: timeouts, user agent, no automatic redirects (the client
//! follows them hop by hop, see [`super::PageClient::get`]) and, for real downloads, DNS that
//! only connects to public addresses.

use std::sync::Arc;
use std::time::Duration;

use mangas_core::domain::Result;
use reqwest::redirect::Policy;
use reqwest::Client;

use super::super::USER_AGENT;
use super::resolver::PublicResolver;

const TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) fn build_http(public_only: bool) -> Result<Client> {
    let builder = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(Policy::none());
    let builder = if public_only {
        builder.dns_resolver(Arc::new(PublicResolver))
    } else {
        builder
    };
    Ok(builder.build()?)
}
