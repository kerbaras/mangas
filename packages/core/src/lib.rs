//! Mangas core: everything the desktop app and the CLI share.
//!
//! Business logic lives here; `apps/desktop` and `apps/cli` only translate input and output.

pub mod domain;
pub mod download;
pub mod export;
pub mod library;
pub mod ml;
pub mod smartcut;
pub mod sources;

pub use domain::{Chapter, Error, Manga, Result};
