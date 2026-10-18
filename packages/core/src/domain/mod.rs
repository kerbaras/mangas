//! Domain types shared by every module.

mod availability;
mod chapter;
mod error;
mod manga;

pub use availability::Availability;
pub use chapter::{compare_numbers, parse_number, sort_by_number, Chapter, ChapterId};
pub use error::{Error, Result};
pub use manga::{Manga, MangaId, MangaStatus};
