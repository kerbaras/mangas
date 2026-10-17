//! The only place that knows the providers: add one by appending its descriptor to
//! [`PROVIDERS`].

use std::sync::Arc;

use futures::future::join_all;
use mangas_core::domain::{Error, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{ProviderStatus, Source};

use crate::provider::{Provider, ProviderInfo};
use crate::{mangadex, mangadot};

const PROVIDERS: &[&dyn Provider] = &[&mangadex::MangaDexProvider, &mangadot::MangaDotProvider];

/// Every registered provider, in display order.
pub fn providers() -> Vec<ProviderInfo> {
    PROVIDERS.iter().map(|p| p.info()).collect()
}

pub fn info(id: &str) -> Option<ProviderInfo> {
    find(id).map(|p| p.info())
}

/// The provider registered as `id`, configured from `settings` (browser session, preferences).
pub fn source(id: &str, settings: &Settings) -> Result<Arc<dyn Source>> {
    find(id).ok_or_else(|| unknown(id))?.build(settings)
}

/// Like [`source`] with every request sent to `base` (see [`Provider::build_at`]).
pub fn source_at(id: &str, settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
    find(id)
        .ok_or_else(|| unknown(id))?
        .build_at(settings, base)
}

/// Whether `id` is usable now: one cheap request at most.
pub async fn status(id: &str, settings: &Settings) -> ProviderStatus {
    match source(id, settings) {
        Ok(source) => source.status().await,
        Err(e) => ProviderStatus::Unreachable {
            reason: e.to_string(),
        },
    }
}

/// Every provider with its status, probed concurrently.
pub async fn statuses(settings: &Settings) -> Vec<(ProviderInfo, ProviderStatus)> {
    let infos = providers();
    let probes = infos.iter().map(|p| status(p.id, settings));
    infos.iter().copied().zip(join_all(probes).await).collect()
}

fn find(id: &str) -> Option<&'static dyn Provider> {
    PROVIDERS.iter().copied().find(|p| p.info().id == id)
}

fn unknown(id: &str) -> Error {
    let ids: Vec<_> = PROVIDERS.iter().map(|p| p.info().id).collect();
    Error::NotFound(format!("source '{id}' (try: {})", ids.join(", ")))
}
