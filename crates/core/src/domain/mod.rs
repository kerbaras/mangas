//! Domain types shared by every module.

mod chapter;
mod error;
mod manga;

pub use chapter::{Chapter, ChapterId};
pub use error::{Error, Result};
pub use manga::{Manga, MangaId, MangaStatus};
