//! The retry loop of [`SiteClient`]: pace, send, read, decide, sleep.

use mangas_core::domain::Result;
use reqwest::RequestBuilder;

use super::super::response::{
    edge_rejected, mitigated, retry_after, retryable_transport, should_retry, Attempt,
};
use super::super::{is_challenge, Reply, RetryPolicy};
use super::SiteClient;

impl SiteClient {
    pub(super) async fn send(
        &self,
        path: &str,
        request: impl Fn() -> RequestBuilder,
        idempotent: bool,
        retry: RetryPolicy,
    ) -> Result<Reply> {
        let mut attempt = 0;
        loop {
            self.bucket.acquire().await;
            match self
                .attempt(path, request(), idempotent, retry, attempt)
                .await
            {
                (_, Some(wait)) => {
                    log::debug!("{}: retry {} in {wait:?}", self.config.info.id, attempt + 1);
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                }
                (result, None) => return result,
            }
        }
    }

    /// One request including its body, so a connection dropped mid-body is retried too. A
    /// response that will be retried is not read, so its `Retry-After` always applies.
    async fn attempt(
        &self,
        path: &str,
        request: RequestBuilder,
        idempotent: bool,
        retry: RetryPolicy,
        attempt: u32,
    ) -> Attempt {
        let id = self.config.info.id;
        let response = match request.send().await {
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
                return (Ok(Reply::empty(status)), Some(wait));
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
            Ok(body) => (Ok(Reply { status, body }), None),
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
