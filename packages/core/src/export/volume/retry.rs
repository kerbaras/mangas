//! "Try again" after a run that stopped part-way: only the books it didn't write, never over them.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use serde::Deserialize;

use super::plan::numbered;
use super::VolumePlan;
use crate::export::kindle::part_path;

/// A book an earlier, stopped run wrote: its chapter ids and the file it was written to (the
/// first part's file when it was split).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KeptBook {
    pub chapters: Vec<String>,
    pub file: String,
}

/// The ids of a planned book's chapters: what identifies the book from one run to the next.
pub fn chapter_ids(plan: &VolumePlan) -> Vec<String> {
    plan.chapters.iter().map(|c| c.id.0.clone()).collect()
}

/// `plans` without the `kept` books, matched by the set of chapter ids. Not by position: a
/// chapter that finished downloading since the stop can add or regroup books ahead of the kept
/// ones. Not by title either, which repeats across same-numbered releases.
pub fn skip_written(plans: Vec<VolumePlan>, kept: &[KeptBook]) -> Vec<VolumePlan> {
    let done: BTreeSet<BTreeSet<&str>> = kept
        .iter()
        .filter(|k| !k.chapters.is_empty())
        .map(|k| k.chapters.iter().map(String::as_str).collect())
        .collect();
    plans
        .into_iter()
        .filter(|plan| {
            let ids: BTreeSet<&str> = plan.chapters.iter().map(|c| c.id.0.as_str()).collect();
            !done.contains(&ids)
        })
        .collect()
}

/// The books a retry writes, as `ext` files: [`skip_written`], then, when replacing, each book
/// numbered ` (2)`, ` (3)`… like `one_file_each`, but never under a kept book's file (nor a name
/// whose first part is one), so Replace can't write over a book the stopped run kept. Without
/// replacing the writer already picks free names, and the kept files are on disk.
pub fn retry_plans(
    plans: Vec<VolumePlan>,
    kept: &[KeptBook],
    ext: &str,
    replace: bool,
) -> Vec<VolumePlan> {
    let left = skip_written(plans, kept);
    if !replace {
        return left;
    }
    let taken: HashSet<String> = kept.iter().map(|k| k.file.to_lowercase()).collect();
    let kept_file = |name: &str| {
        let file = Path::new(name).with_extension(ext);
        let part = part_path(&file, 1);
        [file, part]
            .iter()
            .any(|p| taken.contains(&p.to_string_lossy().to_lowercase()))
    };
    let mut used = HashSet::new();
    let mut free = |name: &str| {
        (1..)
            .map(|n| numbered(name, n))
            .find(|n| !kept_file(n) && used.insert(n.to_lowercase()))
            .unwrap_or_default()
    };
    left.into_iter()
        .map(|p| VolumePlan {
            file_name: free(&p.file_name),
            ..p
        })
        .collect()
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
