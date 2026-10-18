//! Domain types shared by every module.

mod availability;
mod chapter;
mod chapter_group;
mod chapter_tag;
mod error;
mod manga;
mod season_label;

pub use availability::Availability;
pub use chapter::{compare_numbers, parse_number, sort_by_number, Chapter, ChapterId};
pub use chapter_group::{collapse_groups, groups_of, in_group, normalize_group, GroupCount};
pub use chapter_tag::{chapter_tag, season_tag, toc_label};
pub use error::{Error, Result};
pub use manga::{Manga, MangaId, MangaStatus};
pub use season_label::season_label;
