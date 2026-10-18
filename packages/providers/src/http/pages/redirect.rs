//! The page client's HTTP client: timeouts, user agent, and redirects that stay on public hosts.

use std::time::Duration;

use mangas_core::domain::Result;
use mangas_core::download::is_public_http_url;
use reqwest::redirect::{Action, Attempt, Policy};
use reqwest::Client;

use super::super::USER_AGENT;

const TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_REDIRECTS: usize = 10;

pub(super) fn build_http(public_only: bool) -> Result<Client> {
    let redirects = if public_only {
        Policy::custom(public_redirects_only)
    } else {
        Policy::limited(MAX_REDIRECTS)
    };
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .timeout(TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(redirects)
        .build()?)
}

/// Redirects get the same host check as the first request, so a public page URL cannot bounce
/// the downloader onto an internal address.
fn public_redirects_only(attempt: Attempt) -> Action {
    if attempt.previous().len() >= MAX_REDIRECTS {
        attempt.error("too many redirects")
    } else if is_public_http_url(attempt.url().as_str()) {
        attempt.follow()
    } else {
        attempt.error("redirect to a non-public url")
    }
}
