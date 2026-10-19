//! The only place that knows the providers: add one by appending its descriptor to
//! [`PROVIDERS`].

use std::collections::HashMap;
use std::sync::Arc;

use futures::future::join_all;
use mangas_core::domain::{Error, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{ProviderStatus, Source};

use crate::provider::{Provider, ProviderInfo};
use crate::{comick, mangadex, mangadot, tapas, webtoons, weebcentral};

const PROVIDERS: &[&dyn Provider] = &[
    &mangadex::MangaDexProvider,
    &mangadot::MangaDotProvider,
    &weebcentral::WeebCentralProvider,
    &tapas::TapasProvider,
    &comick::ComickProvider,
    &webtoons::WebtoonsProvider,
];

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
    probe(source(id, settings)).await
}

async fn probe(source: Result<Arc<dyn Source>>) -> ProviderStatus {
    match source {
        Ok(source) => source.status().await,
        Err(e) => ProviderStatus::Unreachable {
            reason: e.to_string(),
        },
    }
}

/// Every provider with its status, probed concurrently.
pub async fn statuses(settings: &Settings) -> Vec<(ProviderInfo, ProviderStatus)> {
    statuses_at(settings, &HashMap::new()).await
}

/// Like [`statuses`] with the providers in `bases` (id to base URL) probed there instead (see
/// [`source_at`]).
pub async fn statuses_at(
    settings: &Settings,
    bases: &HashMap<String, String>,
) -> Vec<(ProviderInfo, ProviderStatus)> {
    let infos = providers();
    let probes = infos.iter().map(|p| match bases.get(p.id) {
        Some(base) => probe(source_at(p.id, settings, base)),
        None => probe(source(p.id, settings)),
    });
    infos.iter().copied().zip(join_all(probes).await).collect()
}

pub(crate) fn find(id: &str) -> Option<&'static dyn Provider> {
    PROVIDERS.iter().copied().find(|p| p.info().id == id)
}

pub(crate) fn unknown(id: &str) -> Error {
    let ids: Vec<_> = PROVIDERS.iter().map(|p| p.info().id).collect();
    Error::NotFound(format!("source '{id}' (try: {})", ids.join(", ")))
}
