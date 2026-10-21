//! System DNS lookups with a deadline. A resolver that drops queries costs the deadline once per
//! host: the timeout is remembered for [`REMEMBER`], so a chapter's other pages and their retry
//! fail at once with the same cause instead of each waiting it out.

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::resolver::Refusal;

/// How long a lookup may take: under the connect timeout, so a resolver that never answers
/// fails as a lookup (host named, one retry) instead of as a nameless connect timeout.
pub(super) const LOOKUP_TIMEOUT: Duration = Duration::from_secs(6);

/// How long a host whose lookup timed out fails without asking the resolver again.
const REMEMBER: Duration = Duration::from_secs(30);

/// Hosts whose lookup (or proxy tunnel, see [`super::unreachable`]) timed out, and when.
#[derive(Default)]
pub(super) struct TimedOut(Mutex<HashMap<String, Instant>>);

impl TimedOut {
    pub(super) fn recent(&self, host: &str, now: Instant) -> bool {
        self.0.lock().is_ok_and(|hosts| {
            hosts
                .get(host)
                .is_some_and(|at| now.duration_since(*at) < REMEMBER)
        })
    }

    pub(super) fn note(&self, host: &str, now: Instant) {
        if let Ok(mut hosts) = self.0.lock() {
            hosts.retain(|_, at| now.duration_since(*at) < REMEMBER);
            hosts.insert(host.to_string(), now);
        }
    }
}

/// The addresses `lookup` finds for `host` within `limit`, or why it found none. A host whose
/// lookup timed out less than [`REMEMBER`] ago fails at once, without running `lookup`.
pub(super) async fn bounded<I: Iterator<Item = SocketAddr>>(
    host: &str,
    limit: Duration,
    timed_out: &TimedOut,
    lookup: impl Future<Output = std::io::Result<I>>,
) -> Result<Vec<SocketAddr>, Refusal> {
    let silent = || {
        format!(
            "no answer from the resolver within {} ms",
            limit.as_millis()
        )
    };
    let cause = match timed_out.recent(host, Instant::now()) {
        true => silent(),
        false => match tokio::time::timeout(limit, lookup).await {
            Ok(Ok(addrs)) => return Ok(addrs.collect()),
            Ok(Err(e)) => e.to_string(),
            Err(_) => {
                timed_out.note(host, Instant::now());
                silent()
            }
        },
    };
    Err(Refusal::Unresolved {
        host: host.to_string(),
        cause,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    type Found = std::io::Result<std::vec::IntoIter<SocketAddr>>;

    const SHORT: Duration = Duration::from_millis(50);

    fn found() -> Found {
        Ok(vec![SocketAddr::from(([1, 1, 1, 1], 0))].into_iter())
    }

    #[tokio::test]
    async fn a_resolver_that_never_answers_is_a_named_lookup_failure() {
        let hung = std::future::pending::<Found>();
        let err = bounded("cdn.example", SHORT, &TimedOut::default(), hung)
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "couldn't resolve cdn.example: no answer from the resolver within 50 ms"
        );
    }

    #[tokio::test]
    async fn a_lookup_error_keeps_its_cause() {
        let failed = async {
            Err::<std::vec::IntoIter<SocketAddr>, _>(std::io::Error::other("no such name"))
        };
        let err = bounded("cdn.example", SHORT, &TimedOut::default(), failed)
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "couldn't resolve cdn.example: no such name"
        );
    }

    #[tokio::test]
    async fn a_timed_out_host_fails_at_once_until_it_is_forgotten() {
        let timed_out = TimedOut::default();
        let hung = std::future::pending::<Found>();
        let _ = bounded("cdn.example", SHORT, &timed_out, hung).await;
        let again = bounded("cdn.example", SHORT, &timed_out, async { found() }).await;
        assert_eq!(
            again.unwrap_err().to_string(),
            "couldn't resolve cdn.example: no answer from the resolver within 50 ms"
        );
        let other = bounded("img.example", SHORT, &timed_out, async { found() }).await;
        assert_eq!(other.unwrap().len(), 1);
        let later = Instant::now() + REMEMBER;
        assert!(!timed_out.recent("cdn.example", later));
    }

    #[tokio::test]
    async fn a_lookup_error_is_not_remembered() {
        let timed_out = TimedOut::default();
        let failed =
            async { Err::<std::vec::IntoIter<SocketAddr>, _>(std::io::Error::other("nxdomain")) };
        let _ = bounded("cdn.example", SHORT, &timed_out, failed).await;
        let again = bounded("cdn.example", SHORT, &timed_out, async { found() }).await;
        assert_eq!(again.unwrap().len(), 1);
    }
}
