use std::sync::Arc;

use anyhow::bail;
use clap::{Args as ClapArgs, Subcommand};
use mangas_core::download::queue::{self, Action, Job, JobId, JobState, QueueGate, Scope, Target};
use mangas_core::library;

use super::settled::paused_report;
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
        Command::Ls => return list(&queue::list(&lib)?, library::queue_gate(&lib)?),
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
    match (target, action) {
        (Target::All, Action::Pause) => {
            outln!("Queue paused: nothing new starts until `mangas queue resume --all`")?
        }
        (Target::All, Action::Resume) => outln!("Queue resumed")?,
        _ if changed.is_empty() => outln!("nothing to {}", action.verb())?,
        _ => {}
    }
    Ok(())
}

fn list(jobs: &[Job], gate: QueueGate) -> anyhow::Result<()> {
    if gate.paused {
        outln!("Queue paused (`mangas queue resume --all` resumes it)")?;
    } else if gate.held(queue::now_ms()) {
        outln!("Queue held while the data folder moves")?;
    }
    if jobs.is_empty() {
        outln!("The download queue is empty.")?;
        return Ok(());
    }
    for row in rows(jobs) {
        outln!("{row}")?;
    }
    Ok(())
}

/// One line per job, the state column as wide as its longest entry.
fn rows(jobs: &[Job]) -> Vec<String> {
    let width = jobs.iter().map(|j| state(j).len()).max().unwrap_or(0);
    jobs.iter()
        .map(|job| {
            let error = job.error.as_deref().map(|e| format!("  ({e})"));
            format!(
                "{:>5}  {:<width$} {:>9}  {}{}",
                job.id,
                state(job),
                pages(job),
                label(job),
                error.unwrap_or_default()
            )
        })
        .collect()
}

/// The job's state, flagged when it runs under a dead process's lease.
fn state(job: &Job) -> String {
    match job.stale_lease {
        true => format!("{} (stale lease)", job.state.as_str()),
        false => job.state.as_str().to_string(),
    }
}

fn pages(job: &Job) -> String {
    match job.pages_total {
        0 => "-".to_string(),
        total => format!("{}/{total}", job.pages_done),
    }
}

async fn run_all(lib: Arc<library::Library>) -> anyhow::Result<()> {
    let settled = run_until_settled(lib, Scope::All).await?;
    let jobs = &settled.jobs;
    let failed = count(jobs, JobState::Failed);
    outln!(
        "{} done, {failed} failed, {} paused",
        count(jobs, JobState::Done),
        count(jobs, JobState::Paused)
    )?;
    if let Some((lines, exit)) = paused_report(&settled) {
        for line in lines {
            outln!("{line}")?;
        }
        return Err(exit.into());
    }
    if failed > 0 {
        bail!("{failed} download(s) failed; `mangas queue retry --all` tries them again");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_job_under_a_dead_lease_is_flagged() {
        let job = Job {
            state: JobState::Running,
            ..Default::default()
        };
        assert_eq!(state(&job), "running");
        let stale = Job {
            stale_lease: true,
            ..job
        };
        assert_eq!(state(&stale), "running (stale lease)");
    }

    #[test]
    fn the_state_column_fits_a_stale_lease() {
        let job = |id, stale_lease| Job {
            id,
            state: JobState::Running,
            stale_lease,
            pages_done: 5,
            pages_total: 72,
            ..Default::default()
        };
        let rows = rows(&[job(1, true), job(22, false)]);
        let column = |row: &String| row.find("5/72").unwrap();
        assert_eq!(column(&rows[0]), column(&rows[1]));
        assert!(rows[0].contains("running (stale lease) "));
    }
}
