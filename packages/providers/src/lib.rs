//! Manga providers. Each provider maps one site's API (or its HTML pages, via [`html`]) onto
//! the core [`Source`] trait on top of the common [`http::SiteClient`] / [`http::HostPolicy`];
//! [`registry`] is the only place that knows them by name. See "Provider harness" in `docs/rewrite-design.md`.

pub mod comick;
pub mod doctor;
pub mod html;
pub mod http;
pub mod mangadex;
pub mod mangadot;
mod provider;
mod rate_limit;
mod registry;
mod session;
pub mod tapas;
pub mod webtoons;
pub mod weebcentral;

pub use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};
pub use provider::{Capability, Provider, ProviderInfo};
pub use rate_limit::{host_key, HostLimiters, TokenBucket};
pub use registry::{info, providers, source, source_at, status, statuses};
pub use session::Session;
