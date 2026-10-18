//! The request loop of [`SiteClient`]: follow redirects hop by hop and, per hop, pace, send,
//! read, decide, sleep.

use std::time::{Duration, Instant};

use mangas_core::domain::Result;
use reqwest::{RequestBuilder, Url};

use super::super::redirect::{hop_for, MAX_REDIRECTS};
use super::super::response::{
    edge_rejected, mitigated, retry_after, retryable_transport, should_retry, Attempt,
};
use super::super::{is_challenge, Reply, RetryPolicy};
use super::hop::Outgoing;
use super::SiteClient;

/// How one hop ended: a final reply, or a redirect to follow.
enum Step {
    Reply(Reply),
    Redirect(Outgoing),
}

impl SiteClient {
    pub(super) async fn send(
        &self,
        path: &str,
        first: Outgoing,
        retry: RetryPolicy,
    ) -> Result<Reply> {
        let origin = first.url.clone();
        let mut request = first;
        let mut budget = self.config.timeout;
        for _ in 0..=MAX_REDIRECTS {
            match self
                .send_hop(path, &request, &origin, retry, &mut budget)
                .await?
            {
                Step::Reply(reply) => return Ok(reply),
                Step::Redirect(next) => {
                    let to = next.url.origin().ascii_serialization();
                    log::debug!("{}: {path} redirected to {to}", self.config.info.id);
                    request = next;
                }
            }
        }
        Err(self.error(format!("{path}: more than {MAX_REDIRECTS} redirects")))
    }

    /// One hop with its retries, paced by the bucket of the hop's own host. The hops of a chain
    /// share one `timeout` (`budget`), as when reqwest followed them; a retry starts afresh.
    async fn send_hop(
        &self,
        path: &str,
        request: &Outgoing,
        origin: &Url,
        retry: RetryPolicy,
        budget: &mut Duration,
    ) -> Result<Step> {
        let headers = self.hop_headers(&request.url, origin)?;
        let bucket = self.hop_bucket(&request.url);
        let idempotent = request.idempotent();
        let mut attempt = 0;
        loop {
            bucket.acquire().await;
            if budget.is_zero() {
                return Err(self.error(format!("{path}: timed out following redirects")));
            }
            let builder = self.build(request, headers.clone()).timeout(*budget);
            let started = Instant::now();
            match self
                .attempt(path, builder, request, idempotent, retry, attempt)
                .await
            {
                (_, Some(wait)) => {
                    log::debug!("{}: retry {} in {wait:?}", self.config.info.id, attempt + 1);
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                    *budget = self.config.timeout;
                }
                (result, None) => {
                    *budget = budget.saturating_sub(started.elapsed());
                    return result;
                }
            }
        }
    }

    /// One request including its body, so a connection dropped mid-body is retried too. A
    /// response that will be retried is not read, so its `Retry-After` always applies.
    async fn attempt(
        &self,
        path: &str,
        builder: RequestBuilder,
        request: &Outgoing,
        idempotent: bool,
        retry: RetryPolicy,
        attempt: u32,
    ) -> Attempt<Step> {
        let id = self.config.info.id;
        let response = match builder.send().await {
            Ok(response) => response,
            Err(e) => {
                let wait = retryable_transport(&e, idempotent)
                    .then(|| retry.delay(attempt, None))
                    .flatten();
                if wait.is_none() && attempt > 0 {
                    log::warn!("{id}: giving up on {path} after {}: {e}", attempts(attempt));
                }
                return (Err(e.into()), wait);
            }
        };
        if let Some(hop) = hop_for(&request.method, &response) {
            return (Ok(Step::Redirect(request.clone().follow(hop))), None);
        }
        let status = response.status().as_u16();
        let after = retry_after(&response);
        let json_api = self.config.json_api;
        if should_retry(&response, idempotent, json_api) {
            let edge = edge_rejected(&response, json_api);
            let kind = if edge { " edge error page" } else { "" };
            if let Some(wait) = retry.delay(attempt, after) {
                if edge {
                    log::warn!("{id}: HTTP {status} edge error page for {path}");
                }
                return (Ok(Step::Reply(Reply::empty(status))), Some(wait));
            }
            if attempt > 0 {
                log::warn!(
                    "{id}: giving up on {path} after {}: HTTP {status}{kind}",
                    attempts(attempt)
                );
            }
            if edge {
                let message = format!(
                    "HTTP {status} for {path}: edge error page (HTML, not the API) after {}",
                    attempts(attempt)
                );
                return (Err(self.error(message)), None);
            }
        }
        let mitigated = mitigated(&response);
        let read = response.text().await;
        let wait = || idempotent.then(|| retry.delay(attempt, after)).flatten();
        match read {
            Err(e) => (Err(e.into()), wait()),
            Ok(body) if is_challenge(status, &mitigated, &body) => (Err(self.challenge()), None),
            Ok(body) => (Ok(Step::Reply(Reply { status, body })), None),
        }
    }
}

/// "1 attempt", "3 attempts" for failed attempt number `attempt` (0-based).
fn attempts(attempt: u32) -> String {
    match attempt + 1 {
        1 => "1 attempt".to_string(),
        n => format!("{n} attempts"),
    }
}
