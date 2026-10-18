use clap::Args as ClapArgs;
use mangas_core::domain::Chapter;
use mangas_core::library::updates::{summary, TitleUpdate, UpdatePolicy, UpdateReport};
use mangas_core::library::{self, LOCAL_SOURCE};
use mangas_providers::updates::check_updates;

use super::plural;
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
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if args.source.as_deref() == Some(LOCAL_SOURCE) {
        anyhow::bail!("imported manga have no source to check for new chapters");
    }
    let lib = library::open(&paths::library_db()?)?;
    let policy = UpdatePolicy {
        source: args.source,
        include_finished: args.all,
    };
    let report = check_updates(&lib, &policy).await?;
    if args.json {
        outln!("{}", serde_json::to_string_pretty(&report)?)?;
    } else {
        print_report(&report)?;
    }
    Ok(())
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
