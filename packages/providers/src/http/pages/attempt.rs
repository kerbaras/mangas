//! One page request: send, classify, and decide whether and when to try again.

use std::time::Duration;

use mangas_core::domain::{Error, Result};
use reqwest::{Method, RequestBuilder, Response, Url};

use super::super::redirect::hop_for;
use super::super::response::{mitigated, retry_after, retryable_transport, should_retry};
use super::resolver::refusal_in;
use super::{Fetched, PageClient, Step};

/// The outcome of one attempt and, when it is to be retried, how long to wait first.
type Attempt = (Result<Step>, Option<Duration>);

impl PageClient<'_> {
    /// Rate limiting (429) and server errors retry after `Retry-After` or the backoff; a
    /// challenge page, a refused address and other statuses are final. Transport and body
    /// errors retry too.
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
                if let Some(refusal) = refusal_in(&e) {
                    return (Err(refusal.into_error(id)), None);
                }
                let wait = retryable_transport(&e, true)
                    .then(|| self.retry.delay(attempt, None))
                    .flatten();
                log_failure(id, url, attempt, wait, &e.to_string());
                return (Err(e.into()), wait);
            }
        };
        if let Some(hop) = hop_for(&Method::GET, &response) {
            return (Ok(Step::Redirect(hop.url)), None);
        }
        let status = response.status().as_u16();
        let tries = attempt + 1;
        if should_retry(&response, true, false) {
            let wait = self.retry.delay(attempt, retry_after(&response));
            log_failure(id, url, attempt, wait, &format!("HTTP {status}"));
            if wait.is_some() {
                let body = Vec::new();
                return (
                    Ok(Step::Done(Fetched {
                        status,
                        body,
                        attempts: tries,
                    })),
                    wait,
                );
            }
        }
        if mitigated(&response).eq_ignore_ascii_case("challenge") {
            return (Err(challenge(id, response.url())), None);
        }
        if !response.status().is_success() {
            let body = Vec::new();
            return (
                Ok(Step::Done(Fetched {
                    status,
                    body,
                    attempts: tries,
                })),
                None,
            );
        }
        match self.read(url, response).await {
            Err(Read::TooBig(e)) => (Err(e), None),
            Err(Read::Network(e)) => {
                let wait = self.retry.delay(attempt, None);
                log_failure(id, url, attempt, wait, &e.to_string());
                (Err(e.into()), wait)
            }
            Ok(body) => (
                Ok(Step::Done(Fetched {
                    status,
                    body,
                    attempts: tries,
                })),
                None,
            ),
        }
    }

    /// The body, refused as soon as it is known to exceed `max_bytes` (final, not retried).
    async fn read(&self, url: &str, mut response: Response) -> std::result::Result<Vec<u8>, Read> {
        let Some(max) = self.max_bytes else {
            return Ok(response.bytes().await.map_err(Read::Network)?.to_vec());
        };
        let too_big = || Read::TooBig(self.error(format!("{url} is over {max} bytes")));
        if response.content_length().is_some_and(|n| n > max as u64) {
            return Err(too_big());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(Read::Network)? {
            if body.len() + chunk.len() > max {
                return Err(too_big());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

/// Why a body could not be read.
enum Read {
    TooBig(Error),
    Network(reqwest::Error),
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
