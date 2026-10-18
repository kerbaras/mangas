//! Update checks: re-sync every followed title and report the chapters the library didn't have.
//! `plan` holds the pure parts (which titles, what is new, the summary), `check` the IO edge,
//! `enqueue` hands auto-download titles to the download queue.

mod check;
mod enqueue;
mod plan;

pub use check::{check, Resolve, SourceFor};
pub use enqueue::{enqueue_new, to_download};
pub use plan::{new_chapters, notification, select, summary, Known, Summary, Target, UpdatePolicy};

use serde::{Deserialize, Serialize};

use crate::domain::{Chapter, MangaId};

/// New chapters found for one title.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TitleUpdate {
    pub source: String,
    pub manga_id: MangaId,
    pub title: String,
    /// The title's "download new chapters" flag, so callers know what to queue.
    pub auto_download: bool,
    pub chapters: Vec<Chapter>,
}

/// A title the check could not refresh; the rest of the run is unaffected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFailure {
    pub source: String,
    pub manga_id: MangaId,
    pub title: String,
    pub error: String,
    /// The provider wants a browser session (Cloudflare) before it answers again.
    pub challenge: bool,
}

/// What one run of [`check`] found.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateReport {
    /// Titles with new chapters, by title.
    pub updates: Vec<TitleUpdate>,
    pub failures: Vec<CheckFailure>,
    /// Titles refreshed, failed ones included.
    pub checked: usize,
    /// Followed titles left out by the policy (finished or dropped, local, other source).
    pub skipped: usize,
}
