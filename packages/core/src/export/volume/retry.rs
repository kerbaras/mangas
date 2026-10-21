//! "Try again" after a run that stopped part-way: only the books it didn't write.

use std::collections::BTreeSet;

use super::VolumePlan;

/// The ids of a planned book's chapters: what identifies the book from one run to the next.
pub fn chapter_ids(plan: &VolumePlan) -> Vec<String> {
    plan.chapters.iter().map(|c| c.id.0.clone()).collect()
}

/// `plans` without the books an earlier, stopped run wrote (`written`: each book's chapter ids).
/// Matched by the set of chapter ids, not by position: a chapter that finished downloading since
/// the stop can add or regroup books ahead of the kept ones. Not by title either, which repeats
/// across same-numbered releases. The retry doesn't write the kept books again as ` (2)` copies;
/// replacing or not stays the request's choice.
pub fn skip_written(plans: Vec<VolumePlan>, written: &[Vec<String>]) -> Vec<VolumePlan> {
    let done: BTreeSet<BTreeSet<&str>> = written
        .iter()
        .filter(|ids| !ids.is_empty())
        .map(|ids| ids.iter().map(String::as_str).collect())
        .collect();
    plans
        .into_iter()
        .filter(|plan| {
            let ids: BTreeSet<&str> = plan.chapters.iter().map(|c| c.id.0.as_str()).collect();
            !done.contains(&ids)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Chapter, ChapterId};

    fn plan(file: &str, ids: &[&str]) -> VolumePlan {
        let chapter = |id: &&str| Chapter {
            id: ChapterId(id.to_string()),
            ..Default::default()
        };
        VolumePlan {
            number: None,
            title: file.into(),
            file_name: file.into(),
            chapters: ids.iter().map(chapter).collect(),
        }
    }

    fn ids(written: &[&[&str]]) -> Vec<Vec<String>> {
        let one = |ids: &&[&str]| ids.iter().map(|id| id.to_string()).collect();
        written.iter().map(one).collect()
    }

    fn files(plans: &[VolumePlan]) -> Vec<&str> {
        plans.iter().map(|p| p.file_name.as_str()).collect()
    }

    #[test]
    fn skips_the_written_books_even_when_titles_repeat() {
        let plans = vec![
            plan("Big ch 5", &["a5"]),
            plan("Big ch 5 (2)", &["b5"]),
            plan("Big ch 6", &["a6"]),
        ];
        let left = skip_written(plans.clone(), &ids(&[&["a5"]]));
        assert_eq!(files(&left), ["Big ch 5 (2)", "Big ch 6"]);
        assert_eq!(skip_written(plans.clone(), &[]), plans);
        assert!(skip_written(plans, &ids(&[&["a5"], &["b5"], &["a6"]])).is_empty());
    }

    #[test]
    fn a_book_planned_ahead_of_the_kept_one_since_the_stop_is_written() {
        // The stopped run wrote v1 (ch 1-2) and failed on v2; ch 0 finished downloading after.
        let now = vec![
            plan("v0", &["c0"]),
            plan("v1", &["c2", "c1"]),
            plan("v2", &["c3"]),
        ];
        let left = skip_written(now, &ids(&[&["c1", "c2"]]));
        assert_eq!(files(&left), ["v0", "v2"]);
    }

    #[test]
    fn a_kept_book_that_gained_a_chapter_is_a_different_book() {
        let now = vec![plan("v1", &["c1", "c2", "c2b"]), plan("v2", &["c3"])];
        let left = skip_written(now, &ids(&[&["c1", "c2"], &[]]));
        assert_eq!(files(&left), ["v1", "v2"]);
    }
}
