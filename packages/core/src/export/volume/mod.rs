//! Volume bundling: an ordered set of downloaded chapters of one manga becomes one EPUB with
//! the series' metadata, a cover and a chapter TOC. Planning is pure; `load` reads the library.

mod group;
mod load;
mod meta;
mod name;
mod plan;

use serde::{Deserialize, Serialize};

use crate::domain::Chapter;

pub use group::group;
pub use load::{load, VolumeSource};
pub use meta::{book_meta, chapter_label};
pub use name::{
    file_name, file_name_as, render, series_file_name, DEFAULT_TEMPLATE, MAX_STEM_BYTES,
};
pub use plan::{find_volume, one_file_each, plan, request, written_names, VolumePlan};

/// How a manga's chapters are split into books.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "size", rename_all = "snake_case")]
pub enum Grouping {
    /// One book per provider volume; chapters without one share a trailing book.
    ByVolume,
    /// Consecutive books of this many chapters, numbered from 1.
    Every(usize),
    /// One book per chapter, named after it (`ch 12`, `S2 Ep. 0`) rather than numbered.
    PerChapter,
    /// Everything in one book.
    Single,
}

/// Chapters that go into one book, in reading order, and the volume number it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    pub number: Option<String>,
    pub chapters: Vec<Chapter>,
}
