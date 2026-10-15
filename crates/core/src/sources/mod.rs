//! Manga providers. Each source implements [`Source`]; the rest of the core never talks to a
//! provider directly.

mod source;

pub use source::{PageUrls, Source};

/// Returns the source registered under `name`, if any.
pub fn by_name(name: &str) -> Option<Box<dyn Source>> {
    // Providers register here as they land (mangadex first).
    let _ = name;
    None
}
