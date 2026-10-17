//! Manga providers. Each provider maps one site's API onto the core [`Source`] trait on top of
//! the common [`http::SiteClient`] / [`http::HostPolicy`]; [`registry`] is the only place that
//! knows them by name. See "Provider harness" in `docs/rewrite-design.md`.

pub mod http;
pub mod mangadex;
pub mod mangadot;
mod provider;
mod rate_limit;
mod registry;
mod session;

pub use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};
pub use provider::{Capability, Provider, ProviderInfo};
pub use rate_limit::TokenBucket;
pub use registry::{info, providers, source, status, statuses};
pub use session::Session;
