//! Search across the providers the user has enabled (see [`mangas_core::sources::search`]).

use std::sync::Arc;

use mangas_core::library::Settings;
use mangas_core::sources::search::{group, in_order};
use mangas_core::sources::{search_all, SearchGroup, Source, SEARCH_TIMEOUT};

use crate::provider::{Capability, ProviderInfo};
use crate::registry::{providers, source};

/// Providers a search runs on: enabled in `settings`, able to search, and listed in `only`
/// unless it is empty. Registry order.
pub fn searchable(settings: &Settings, only: &[String]) -> Vec<ProviderInfo> {
    providers()
        .into_iter()
        .filter(|p| p.capabilities.contains(&Capability::Search))
        .filter(|p| settings.provider_enabled(p.id))
        .filter(|p| only.is_empty() || only.iter().any(|id| id == p.id))
        .collect()
}

/// Searches every [`searchable`] provider concurrently, one group each in registry order. A
/// provider that cannot be built gets an unreachable group like one that fails to answer.
pub async fn search_enabled(
    settings: &Settings,
    query: &str,
    limit: usize,
    only: &[String],
) -> Vec<SearchGroup> {
    let infos = searchable(settings, only);
    let mut built: Vec<Arc<dyn Source>> = Vec::new();
    let mut failed = Vec::new();
    for info in &infos {
        match source(info.id, settings) {
            Ok(s) => built.push(s),
            Err(e) => failed.push(group(info.id, Err(e))),
        }
    }
    let mut groups = search_all(&built, query, limit, SEARCH_TIMEOUT, &settings.network()).await;
    groups.extend(failed);
    let ids: Vec<&str> = infos.iter().map(|p| p.id).collect();
    in_order(groups, &ids)
}
