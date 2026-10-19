use std::sync::Arc;

use clap::Args as ClapArgs;
use mangas_core::download::queue::{JobState, Scope, Settled};
use mangas_core::library::updates::{enqueue_new, summary, Summary, UpdatePolicy, UpdateReport};
use mangas_core::library::{self, LOCAL_SOURCE};
use mangas_providers::updates::check_updates;
use serde::Serialize;

use super::plural;
use super::settled::{failed_note, paused_report};
use super::update_print::print_report;
use super::watch::{count, run_quietly, run_until_settled};
use crate::exit::Exit;
use crate::output::{outln, print_json};
use crate::paths;

/// Exit code when some titles couldn't be checked (the rest of the report is still printed).
pub const NOT_CHECKED_EXIT: u8 = 2;

#[derive(ClapArgs)]
pub struct Args {
    /// Only check titles of this provider
    #[arg(long)]
    pub source: Option<String>,
    /// Also check titles marked completed or dropped
    #[arg(long)]
    pub all: bool,
    /// Print the report as JSON
    #[arg(long)]
    pub json: bool,
    /// Download every new chapter found, not only those of titles with auto-download on
    #[arg(long)]
    pub download: bool,
}

/// `--json` output: the report, its totals and what the download queue did with it.
#[derive(Serialize)]
struct JsonReport<'a> {
    #[serde(flatten)]
    report: &'a UpdateReport,
    summary: Summary,
    downloaded: usize,
    failed: usize,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    match args.source.as_deref() {
        Some(LOCAL_SOURCE) => {
            anyhow::bail!("imported manga have no source to check for new chapters")
        }
        Some(id) if mangas_providers::info(id).is_none() => {
            let ids: Vec<_> = mangas_providers::providers()
                .into_iter()
                .map(|p| p.id)
                .collect();
            anyhow::bail!("unknown provider '{id}' (try: {})", ids.join(", "))
        }
        _ => {}
    }
    let lib = Arc::new(library::open(&paths::library_db()?)?);
    let policy = UpdatePolicy {
        source: args.source,
        include_finished: args.all,
    };
    let report = check_updates(&lib, &policy).await?;
    if !args.json {
        print_report(&report)?;
    }
    let settled = download(lib, &report, args.download, args.json).await?;
    let jobs = &settled.jobs;
    let failed = count(jobs, JobState::Failed);
    if args.json {
        let out = JsonReport {
            report: &report,
            summary: summary(&report),
            downloaded: count(jobs, JobState::Done),
            failed,
        };
        print_json(&out)?;
    }
    let failed_note = failed_note(jobs, "chapter");
    if let Some((lines, exit)) = paused_report(&settled) {
        if !args.json {
            for line in lines {
                outln!("{line}")?;
            }
        }
        if let Some(note) = &failed_note {
            eprintln!("{note}");
        }
        return Err(exit.into());
    }
    if let Some(note) = failed_note {
        anyhow::bail!("{note}");
    }
    match not_checked(&report) {
        Some(exit) => Err(exit.into()),
        None => Ok(()),
    }
}

/// The error for a run where some titles couldn't be checked, naming them.
fn not_checked(report: &UpdateReport) -> Option<Exit> {
    let names: Vec<String> = report
        .failures
        .iter()
        .map(|f| format!("{} ({})", f.title, f.source))
        .collect();
    (!names.is_empty()).then(|| Exit {
        code: NOT_CHECKED_EXIT,
        message: format!(
            "not checked: {}; the other titles were checked",
            names.join(", ")
        ),
    })
}

/// Queues the new chapters of auto-download titles (all of them with `everything`) and runs
/// the queue until they settle.
async fn download(
    lib: Arc<library::Library>,
    report: &UpdateReport,
    everything: bool,
    quiet: bool,
) -> anyhow::Result<Settled> {
    let jobs = enqueue_new(&lib, report, everything)?;
    if jobs.is_empty() {
        return Ok(Settled {
            jobs: Vec::new(),
            paused: false,
        });
    }
    let scope = Scope::Jobs(jobs.into_iter().collect());
    if quiet {
        return run_quietly(lib, scope).await;
    }
    outln!(
        "Downloading {}",
        plural(scope_len(&scope), "chapter", "chapters")
    )?;
    run_until_settled(lib, scope).await
}

fn scope_len(scope: &Scope) -> usize {
    match scope {
        Scope::Jobs(ids) => ids.len(),
        Scope::All => 0,
    }
}
