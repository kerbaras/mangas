//! Manga providers. Each provider implements the core [`Source`] trait on top of the shared
//! [`http::ApiClient`]; [`registry`] is the only place that knows them by name.

pub mod http;
pub mod mangadex;
pub mod mangadot;
mod rate_limit;
mod registry;
mod session;

pub use mangas_core::sources::{Header, PageUrls, Source};
pub use rate_limit::TokenBucket;
pub use registry::{by_name, info, ProviderInfo, PROVIDERS};
pub use session::Session;
