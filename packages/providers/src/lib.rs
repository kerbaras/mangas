//! Manga providers. Each provider maps one site's API (or its HTML pages, via [`html`]) onto
//! the core [`Source`] trait on top of the common [`http::SiteClient`] / [`http::HostPolicy`];
//! [`registry`] is the only place that knows them by name. See "Provider harness" in `docs/rewrite-design.md`.

pub mod comick;
pub mod covers;
pub mod doctor;
pub mod html;
pub mod http;
mod languages;
pub mod mangadex;
pub mod mangadot;
pub mod options;
mod provider;
mod queue_runner;
mod rate_limit;
mod registry;
mod search;
mod session;
pub mod tapas;
pub mod updates;
pub mod webtoons;
pub mod weebcentral;

pub use languages::{language_for, languages_for, offered_language, resolve_languages};
pub use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};
pub use provider::{Capability, Provider, ProviderInfo};
pub use queue_runner::{QueueRunner, RootFor};
pub use rate_limit::{host_key, HostLimiters, TokenBucket};
pub use registry::{info, providers, source, source_at, status, statuses, statuses_at};
pub use search::{search_enabled, searchable};
pub use session::Session;
