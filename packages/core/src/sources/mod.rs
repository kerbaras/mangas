//! Provider contract. Implementations live in the `mangas-providers` crate; the rest of the
//! core only ever sees a [`Source`].

mod source;

pub use source::{Header, PageUrls, Source};
