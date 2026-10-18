//! Provider contract. Implementations live in the `mangas-providers` crate; the rest of the
//! core only ever sees a [`Source`].

pub mod search;
mod source;
mod status;

pub use search::{search_all, SearchGroup, SEARCH_TIMEOUT};
pub use source::{Header, PageUrls, Source};
pub use status::ProviderStatus;
