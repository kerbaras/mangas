//! Provider contract. Implementations live in the `mangas-providers` crate; the rest of the
//! core only ever sees a [`Source`].

mod source;
mod status;

pub use source::{Header, PageUrls, Source};
pub use status::ProviderStatus;
