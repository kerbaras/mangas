pub mod add;
pub mod chapters;
pub mod download;
pub mod import;
pub mod info;
pub mod kindle;
pub mod list;
pub mod providers;
pub mod remove;
pub mod search;
pub mod settings;

use std::sync::Arc;

use mangas_core::library::{self, Settings};
use mangas_providers::Source;

use crate::paths;

/// The provider called `name`, carrying the browser session the desktop app stored for it.
pub fn source_by_name(name: &str) -> anyhow::Result<Arc<dyn Source>> {
    Ok(mangas_providers::source(name, &stored_settings())?)
}

/// App settings, or the defaults when the library cannot be read.
pub fn stored_settings() -> Settings {
    paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| Ok(library::get_settings(&lib)?))
        .unwrap_or_default()
}
