//! mangas core: everything the desktop app and the CLI share.
//!
//! Business logic lives here; `apps/desktop` and `apps/cli` only translate input and output.

pub mod app_update;
pub mod backup;
pub mod covers;
pub mod diagnostics;
pub mod domain;
pub mod download;
pub mod export;
pub mod images;
pub mod library;
pub mod logging;
pub mod ml;
pub mod net;
pub mod send;
pub mod smartcut;
pub mod sources;
pub mod storage;
mod zip_out;

pub use domain::{Chapter, Error, Manga, Result};
