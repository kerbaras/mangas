//! The download queue's [`JobRunner`] for the apps: the job's provider (with the session
//! stored in the settings), the shared page client, and the downloads folder the settings pick.

use std::path::PathBuf;

use async_trait::async_trait;
use mangas_core::domain::Result;
use mangas_core::download::queue::{run_job, Job, JobRunner};
use mangas_core::download::OnEvent;
use mangas_core::library::{self, Library, Settings};

use crate::http::PageClient;

/// Downloads folder for the given settings (each app knows its own data folder rules).
pub type RootFor = dyn Fn(&Settings) -> Result<PathBuf> + Send + Sync;

pub struct QueueRunner {
    root_for: Box<RootFor>,
}

impl QueueRunner {
    pub fn new(root_for: Box<RootFor>) -> Self {
        QueueRunner { root_for }
    }
}

#[async_trait]
impl JobRunner for QueueRunner {
    /// Settings are read per job, so a new session or data folder applies to the next chapter.
    async fn run(&self, lib: &Library, job: &Job, on_event: OnEvent<'_>) -> Result<()> {
        let settings = library::get_settings(lib)?;
        let source = crate::source(&job.source, &settings)?;
        let pages = PageClient::for_source(source.as_ref())?;
        let root = (self.root_for)(&settings)?;
        run_job(lib, source.as_ref(), &pages, job, &root, on_event).await?;
        Ok(())
    }
}
