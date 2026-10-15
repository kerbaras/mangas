use serde::{Deserialize, Serialize};

use crate::domain::ChapterId;

/// Progress notification. Serialised as `{"kind": "page_done", ...}` for the desktop app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    PageDone {
        chapter: ChapterId,
        /// 1-based page index.
        page: usize,
        total: usize,
    },
    ChapterDone {
        chapter: ChapterId,
        path: String,
        pages: usize,
    },
    Error {
        chapter: ChapterId,
        message: String,
    },
}

/// Progress sink. Called from several tasks at once, so it must be `Sync`.
pub type OnEvent<'a> = &'a (dyn Fn(Event) + Send + Sync);
