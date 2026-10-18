use std::sync::Arc;

use anyhow::bail;
use clap::{Args as ClapArgs, Subcommand};
use mangas_core::download::queue::{self, Action, Job, JobId, JobState, Scope, Target};
use mangas_core::library;

use super::watch::{count, label, run_until_settled};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the download queue
    Ls,
    /// Pause a download (it keeps its pages and resumes later)
    Pause(Which),
    /// Resume a paused download
    Resume(Which),
    /// Cancel a download, or dismiss a failed one (pages already on disk stay there)
    Cancel(Which),
    /// Queue a failed or cancelled download again
    Retry(Which),
    /// Remove finished and cancelled downloads from the list
    Clear,
    /// Download everything queued now, then exit
    Run,
}

#[derive(ClapArgs)]
#[group(required = true, multiple = false)]
struct Which {
    /// Download id, as shown by `mangas queue ls`
    id: Option<JobId>,
    /// Every download this applies to
    #[arg(long)]
    all: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let lib = Arc::new(library::open(&paths::library_db()?)?);
    let action = match args.command {
        Command::Ls => return list(&queue::list(&lib)?),
        Command::Clear => {
            let removed = queue::clear_finished(&lib)?;
            return Ok(outln!("removed {removed} download(s)")?);
        }
        Command::Run => return run_all(lib).await,
        Command::Pause(w) => (w, Action::Pause),
        Command::Resume(w) => (w, Action::Resume),
        Command::Cancel(w) => (w, Action::Cancel),
        Command::Retry(w) => (w, Action::Retry),
    };
    let (which, action) = action;
    let target = match which.id {
        Some(id) if !which.all => Target::Job(id),
        _ => Target::All,
    };
    let changed = queue::control(&lib, target, action)?;
    for job in &changed {
        outln!("{:>5}  {}  {}", job.id, job.state.as_str(), label(job))?;
    }
    if changed.is_empty() {
        outln!("nothing to {}", action.verb())?;
    }
    Ok(())
}

fn list(jobs: &[Job]) -> anyhow::Result<()> {
    if jobs.is_empty() {
        outln!("The download queue is empty.")?;
        return Ok(());
    }
    for job in jobs {
        outln!(
            "{:>5}  {:<9} {:>9}  {}{}",
            job.id,
            job.state.as_str(),
            pages(job),
            label(job),
            job.error
                .as_deref()
                .map(|e| format!("  ({e})"))
                .unwrap_or_default()
        )?;
    }
    Ok(())
}

fn pages(job: &Job) -> String {
    match job.pages_total {
        0 => "-".to_string(),
        total => format!("{}/{total}", job.pages_done),
    }
}

async fn run_all(lib: Arc<library::Library>) -> anyhow::Result<()> {
    let jobs = run_until_settled(lib, Scope::All).await?;
    let failed = count(&jobs, JobState::Failed);
    outln!(
        "{} done, {failed} failed, {} paused",
        count(&jobs, JobState::Done),
        count(&jobs, JobState::Paused)
    )?;
    if failed > 0 {
        bail!("{failed} download(s) failed; `mangas queue retry --all` tries them again");
    }
    Ok(())
}
