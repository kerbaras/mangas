use serde::{Deserialize, Serialize};

use crate::domain::{ChapterId, MangaId};

/// Row id of a [`Job`] in `download_jobs`.
pub type JobId = i64;

/// Where a job is in its life: `Queued` → `Running` → `Done`, with `Paused`, `Failed` and
/// `Cancelled` on the side. `Done`, `Failed` and `Cancelled` are final until re-queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Paused,
    Done,
    Failed,
    Cancelled,
}

impl JobState {
    pub const ALL: [JobState; 6] = [
        JobState::Queued,
        JobState::Running,
        JobState::Paused,
        JobState::Done,
        JobState::Failed,
        JobState::Cancelled,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Paused => "paused",
            JobState::Done => "done",
            JobState::Failed => "failed",
            JobState::Cancelled => "cancelled",
        }
    }

    pub fn parse(raw: &str) -> Option<JobState> {
        JobState::ALL.into_iter().find(|s| s.as_str() == raw)
    }

    /// Queued, running or paused: the chapter is still on its way.
    pub fn is_active(self) -> bool {
        matches!(
            self,
            JobState::Queued | JobState::Running | JobState::Paused
        )
    }
}

/// One chapter in the download queue. The display fields (`manga_title`, `number`, ...) come
/// from the library when the job is read and are never written back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub source: String,
    pub manga_id: MangaId,
    pub chapter_id: ChapterId,
    pub state: JobState,
    /// Attempts started so far; reset by an explicit retry or a new enqueue.
    pub attempts: u32,
    /// Last failure, kept while a retry waits so the user sees why.
    pub error: Option<String>,
    pub pages_done: usize,
    /// 0 until the source has listed the chapter's pages.
    pub pages_total: usize,
    /// Unix ms before which a queued job must not start (retry backoff).
    pub not_before: i64,
    /// Process that runs the job while `Running`.
    pub owner: Option<String>,
    /// Unix ms of the owner's last sign of life.
    pub heartbeat_at: i64,
    /// Running, but the owner's lease expired (its process died): reclaimed by the next worker
    /// step. Set by [`super::list`] when the jobs are read, never stored.
    #[serde(default)]
    pub stale_lease: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub manga_title: String,
    pub number: String,
    pub title: Option<String>,
    pub language: String,
    /// Chapter directory once downloaded.
    pub local_path: Option<String>,
}

impl Default for Job {
    fn default() -> Self {
        Job {
            id: 0,
            source: String::new(),
            manga_id: MangaId(String::new()),
            chapter_id: ChapterId(String::new()),
            state: JobState::Queued,
            attempts: 0,
            error: None,
            pages_done: 0,
            pages_total: 0,
            not_before: 0,
            owner: None,
            heartbeat_at: 0,
            stale_lease: false,
            created_at: 0,
            updated_at: 0,
            manga_title: String::new(),
            number: String::new(),
            title: None,
            language: String::new(),
            local_path: None,
        }
    }
}

/// The queue-wide gate shared by every process: no job is claimed while it is closed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueGate {
    /// The user's "pause all"; only "resume all" clears it.
    pub paused: bool,
    /// Unix ms until which a storage move holds the queue (0: not held).
    pub held_until: i64,
}

impl QueueGate {
    pub fn held(&self, now: i64) -> bool {
        self.held_until > now
    }

    pub fn closed(&self, now: i64) -> bool {
        self.paused || self.held(now)
    }
}
