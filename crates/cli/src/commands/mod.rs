pub mod add;
pub mod chapters;
pub mod download;
pub mod kindle;
pub mod list;
pub mod remove;
pub mod search;

use anyhow::bail;
use mangas_core::sources::{self, Source};

pub fn source_by_name(name: &str) -> anyhow::Result<Box<dyn Source>> {
    match sources::by_name(name) {
        Some(source) => Ok(source),
        None => bail!("unknown source '{name}'"),
    }
}
