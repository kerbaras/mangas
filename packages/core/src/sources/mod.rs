//! Manga providers. Each source implements [`Source`]; the rest of the core never talks to a
//! provider directly.

pub mod mangadex;
mod source;

pub use source::{PageUrls, Source};

/// Returns the source registered under `name`, if any.
pub fn by_name(name: &str) -> Option<Box<dyn Source>> {
    match name {
        mangadex::SOURCE_NAME => mangadex::MangaDex::new()
            .ok()
            .map(|s| Box::new(s) as Box<dyn Source>),
        _ => None,
    }
}
