//! One page request: send, classify, and decide whether and when to try again.

use std::time::Duration;

use mangas_core::domain::{Error, Result};
use mangas_core::download::is_public_http_url;
use reqwest::{RequestBuilder, Url};

use super::super::response::{mitigated, retry_after, retryable_transport, should_retry};
use super::{Fetched, PageClient};

/// The outcome of one attempt and, when it is to be retried, how long to wait first.
type Attempt = (Result<Fetched>, Option<Duration>);

impl PageClient<'_> {
    /// Rate limiting (429) and server errors retry after `Retry-After` or the backoff; a
    /// challenge page and other statuses are final. Transport and body errors retry too.
    pub(super) async fn attempt(
        &self,
        url: &str,
        request: RequestBuilder,
        attempt: u32,
    ) -> Attempt {
        let id = self.source.name();
        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                let wait = retryable_transport(&e, true)
                    .then(|| self.retry.delay(attempt, None))
                    .flatten();
                log_failure(id, url, attempt, wait, &e.to_string());
                return (Err(e.into()), wait);
            }
        };
        if self.public_only && !is_public_http_url(response.url().as_str()) {
            let message = format!("refusing response from non-public url {}", response.url());
            return (Err(self.error(message)), None);
        }
        let status = response.status().as_u16();
        let tries = attempt + 1;
        if should_retry(&response, true, false) {
            let wait = self.retry.delay(attempt, retry_after(&response));
            log_failure(id, url, attempt, wait, &format!("HTTP {status}"));
            if wait.is_some() {
                let body = Vec::new();
                return (
                    Ok(Fetched {
                        status,
                        body,
                        attempts: tries,
                    }),
                    wait,
                );
            }
        }
        if mitigated(&response).eq_ignore_ascii_case("challenge") {
            return (Err(challenge(id, response.url())), None);
        }
        match response.bytes().await {
            Err(e) => {
                let wait = self.retry.delay(attempt, None);
                log_failure(id, url, attempt, wait, &e.to_string());
                (Err(e.into()), wait)
            }
            Ok(body) => (
                Ok(Fetched {
                    status,
                    body: body.to_vec(),
                    attempts: tries,
                }),
                None,
            ),
        }
    }
}

/// A verification page: the same [`Error::Challenge`] as `SiteClient`, so the app offers
/// "Connect" for the page's host.
fn challenge(id: &str, url: &Url) -> Error {
    Error::Challenge {
        name: id.to_string(),
        site: url.origin().ascii_serialization(),
    }
}

fn log_failure(id: &str, url: &str, attempt: u32, wait: Option<Duration>, what: &str) {
    match wait {
        Some(wait) => log::warn!(
            "{id}: {what} for page {url}; retry {} in {wait:?}",
            attempt + 1
        ),
        None if attempt > 0 => {
            log::warn!(
                "{id}: giving up on page {url} after {}: {what}",
                attempts(attempt + 1)
            );
        }
        None => {}
    }
}

/// "1 attempt", "3 attempts".
pub(super) fn attempts(count: u32) -> String {
    match count {
        1 => "1 attempt".to_string(),
        n => format!("{n} attempts"),
    }
}
