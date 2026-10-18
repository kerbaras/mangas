//! The human-readable `mangas update` report.

use std::collections::BTreeSet;

use mangas_core::domain::Chapter;
use mangas_core::library::updates::{summary, TitleUpdate, UpdateReport};

use super::plural;
use crate::output::outln;

/// One line per title with new chapters, one per title not checked, then the totals.
pub fn print_report(report: &UpdateReport) -> anyhow::Result<()> {
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
    if !report.failures.is_empty() {
        let sources: BTreeSet<&str> = report.failures.iter().map(|f| f.source.as_str()).collect();
        let sources: Vec<&str> = sources.into_iter().collect();
        tail.push(format!(
            "{} not checked on {}",
            plural(report.failures.len(), "title", "titles"),
            sources.join(", ")
        ));
    }
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
