use std::sync::Arc;

use clap::Args as ClapArgs;
use mangas_core::domain::Chapter;
use mangas_core::download::queue::{JobState, Scope, Settled};
use mangas_core::library::updates::{
    enqueue_new, summary, Summary, TitleUpdate, UpdatePolicy, UpdateReport,
};
use mangas_core::library::{self, LOCAL_SOURCE};
use mangas_providers::updates::check_updates;
use serde::Serialize;

use super::plural;
use super::watch::{count, run_quietly, run_until_settled};
use crate::output::outln;
use crate::paths;

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
    if args.source.as_deref() == Some(LOCAL_SOURCE) {
        anyhow::bail!("imported manga have no source to check for new chapters");
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
        outln!("{}", serde_json::to_string_pretty(&out)?)?;
    }
    if failed > 0 {
        anyhow::bail!("{failed} chapter(s) failed; `mangas queue retry --all` tries them again");
    }
    Ok(())
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

fn print_report(report: &UpdateReport) -> anyhow::Result<()> {
    for update in &report.updates {
        outln!("{}", title_line(update))?;
    }
    for f in &report.failures {
        let hint = if f.challenge {
            " (connect it in the app's Settings)"
        } else {
            ""
        };
        outln!("Not checked: {} ({}): {}{hint}", f.title, f.source, f.error)?;
    }
    let mut tail = vec![format!(
        "checked {}",
        plural(report.checked, "title", "titles")
    )];
    if report.skipped > 0 {
        tail.push(format!(
            "skipped {} (completed, dropped or imported; --all checks them)",
            report.skipped
        ));
    }
    outln!("{} ({})", summary(report).line(), tail.join(", "))?;
    Ok(())
}

fn title_line(update: &TitleUpdate) -> String {
    let mut labels: Vec<String> = Vec::new();
    for label in update.chapters.iter().map(label) {
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    format!(
        "{} ({}): {} new: {}",
        update.title,
        update.source,
        labels.len(),
        labels.join(", ")
    )
}

fn label(chapter: &Chapter) -> String {
    match (chapter.number.trim(), chapter.title.as_deref()) {
        ("", Some(title)) => title.to_string(),
        ("", None) => chapter.id.0.clone(),
        (number, _) => format!("ch. {number}"),
    }
}
