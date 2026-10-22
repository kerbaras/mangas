//! The export plan: each book's file with its estimated size against the delivery's budget,
//! and, for a book over it, the flags that make it fit. Printed before anything is written.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::{plan_budget, BookBudget, BudgetSpec, Delivery, Levers};
use mangas_core::export::volume::VolumePlan;
use serde::Serialize;

use crate::output::outln;

/// One planned book in the `--json` output (`plan`), its budget fields flattened in.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedBook {
    pub path: PathBuf,
    pub title: String,
    pub chapters: usize,
    #[serde(flatten)]
    pub budget: BookBudget,
}

/// Each of `plans` (named `names` in `dir`) sized against `spec`.
pub fn size(
    plans: &[VolumePlan],
    names: &[String],
    dir: &Path,
    spec: BudgetSpec,
) -> anyhow::Result<Vec<PlannedBook>> {
    let books: Vec<Vec<PathBuf>> = plans.iter().map(chapter_dirs).collect();
    let budgets = plan_budget(&books, spec)?;
    Ok((plans.iter().zip(names).zip(budgets))
        .map(|((p, name), budget)| PlannedBook {
            path: dir.join(name),
            title: p.title.clone(),
            chapters: p.chapters.len(),
            budget,
        })
        .collect())
}

fn chapter_dirs(p: &VolumePlan) -> Vec<PathBuf> {
    (p.chapters.iter())
        .filter_map(|c| c.local_path.as_ref().map(PathBuf::from))
        .collect()
}

/// The plan as text: a line per book, then its size against the budget and what makes it fit.
pub fn print(
    plans: &[VolumePlan],
    plan: &[PlannedBook],
    first_parts: &[String],
    split_mb: Option<u64>,
) -> anyhow::Result<()> {
    for ((p, book), first_part) in plans.iter().zip(plan).zip(first_parts) {
        let (first, last) = match (p.chapters.first(), p.chapters.last()) {
            (Some(f), Some(l)) => (f.number.as_str(), l.number.as_str()),
            _ => ("", ""),
        };
        let parts = match split_mb {
            Some(mb) => format!(
                ", in parts of {mb} MB if bigger: {}, …",
                book.path.with_file_name(first_part).display()
            ),
            None => String::new(),
        };
        let chapters = book.chapters;
        outln!(
            "{}  ({chapters} chapters, {first}–{last}{parts})",
            book.path.display()
        )?;
        outln!("  {}", budget_line(&book.budget))?;
        if let Some(levers) = &book.budget.levers {
            outln!("  {}", levers_line(levers))?;
        }
    }
    Ok(())
}

/// `{"plan": [...]}`: the `--dry-run --json` output.
pub fn print_json(plan: &[PlannedBook]) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(&serde_json::json!({ "plan": plan }))?;
    outln!("{json}")?;
    Ok(())
}

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1e6)
}

/// `about 212.0 MB of the 200 MB web budget, JPEG quality 90: over budget`.
pub fn budget_line(b: &BookBudget) -> String {
    let limit = match b.budget_bytes {
        n if n == Delivery::Email.budget() => "Send-to-Kindle email",
        _ => "Send-to-Kindle web",
    };
    let files = match b.files {
        1 => String::new(),
        n => format!(" in {n} parts"),
    };
    let verdict = if b.over_budget { ": over budget" } else { "" };
    format!(
        "about {}{files} for the {} MB {limit} budget, {} pages, JPEG quality {}{verdict}",
        mb(b.estimated_bytes),
        b.budget_bytes / 1_000_000,
        b.pages,
        b.jpeg_quality
    )
}

/// `to fit: --every 8 (fewer chapters per book), --split-mb 180 (parts) or --quality 60`.
pub fn levers_line(l: &Levers) -> String {
    let mut ways = Vec::new();
    if let Some(n) = l.chapters_per_book {
        ways.push(format!("--every {n} (fewer chapters per book)"));
    }
    ways.push(format!("--split-mb {} (parts)", l.split_bytes / 1_000_000));
    match l.jpeg_quality {
        Some(q) => ways.push(format!("--quality {q} (lower JPEG quality)")),
        None => ways.push("no JPEG quality down to 50 fits".into()),
    }
    format!("to fit: {}", ways.join(", "))
}

#[cfg(test)]
#[path = "export_plan_tests.rs"]
mod tests;
