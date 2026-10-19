//! Search across several providers at once. Each provider gets the same deadline and its own
//! result group, so one that fails, times out or wants a browser session never sinks the rest.

use crate::library::Network;
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use serde::Serialize;

use super::{ProviderStatus, Source};
use crate::domain::{Error, Manga, Result};

/// How long one provider may take to answer a search before it counts as unreachable.
pub const SEARCH_TIMEOUT: Duration = Duration::from_secs(15);

/// One provider's share of a search: its results when it answered (`Ready`), otherwise why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchGroup {
    pub provider: String,
    pub status: ProviderStatus,
    pub items: Vec<Manga>,
}

impl SearchGroup {
    pub fn is_ok(&self) -> bool {
        self.status == ProviderStatus::Ready
    }
}

/// Searches every source concurrently, each bounded by `timeout`. Groups come back in the order
/// of `sources`; an empty query gives empty groups without any request.
pub async fn search_all(
    sources: &[Arc<dyn Source>],
    query: &str,
    limit: usize,
    timeout: Duration,
    network: &Network,
) -> Vec<SearchGroup> {
    let query = query.trim();
    join_all(sources.iter().map(|source| async move {
        if query.is_empty() {
            return group(source.name(), Ok(Vec::new()));
        }
        match tokio::time::timeout(timeout, source.search(query, limit)).await {
            Ok(outcome) => group(source.name(), outcome),
            Err(_) => timed_out(source.name(), timeout, network),
        }
    }))
    .await
}

/// The group for `provider` from its search outcome: a challenge means it needs a session,
/// any other error that it is unreachable. Results repeating an id are dropped.
pub fn group(provider: &str, outcome: Result<Vec<Manga>>) -> SearchGroup {
    let (status, items) = match outcome {
        Ok(items) => (ProviderStatus::Ready, unique(items)),
        Err(Error::Challenge { .. }) => (ProviderStatus::NeedsSession, Vec::new()),
        Err(e) => (
            ProviderStatus::Unreachable {
                reason: e.to_string(),
            },
            Vec::new(),
        ),
    };
    SearchGroup {
        provider: provider.to_string(),
        status,
        items,
    }
}

/// The group of a provider with no answer within `after`, naming the proxy it went through.
pub fn timed_out(provider: &str, after: Duration, network: &Network) -> SearchGroup {
    SearchGroup {
        provider: provider.to_string(),
        status: ProviderStatus::Unreachable {
            reason: crate::net::no_answer(network, after.as_secs()),
        },
        items: Vec::new(),
    }
}

/// Puts `groups` in the order of `ids` (registry order); unknown providers go last.
pub fn in_order(mut groups: Vec<SearchGroup>, ids: &[&str]) -> Vec<SearchGroup> {
    groups.sort_by_key(|g| {
        ids.iter()
            .position(|id| *id == g.provider)
            .unwrap_or(ids.len())
    });
    groups
}

/// Results across every group that answered.
pub fn total(groups: &[SearchGroup]) -> usize {
    groups.iter().map(|g| g.items.len()).sum()
}

fn unique(items: Vec<Manga>) -> Vec<Manga> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|m| seen.insert(m.id.0.clone()))
        .collect()
}
