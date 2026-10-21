//! "Try again" after a run that stopped part-way: only the books it didn't write.

use super::VolumePlan;

/// `plans` without the first `written`: the books an earlier, stopped run of the same plan wrote.
/// A run stops at its first failure, so the books it kept always lead the plan; counting them
/// (not matching titles, which repeat across same-numbered releases) skips exactly those. The
/// retry doesn't write them again as ` (2)` copies; replacing or not stays the request's choice.
pub fn skip_written(plans: Vec<VolumePlan>, written: usize) -> Vec<VolumePlan> {
    plans.into_iter().skip(written).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(title: &str, file: &str) -> VolumePlan {
        VolumePlan {
            number: None,
            title: title.into(),
            file_name: file.into(),
            chapters: Vec::new(),
        }
    }

    #[test]
    fn skips_the_written_books_even_when_titles_repeat() {
        let plans = vec![
            plan("Big ch 5", "Big ch 5"),
            plan("Big ch 5", "Big ch 5 (2)"),
            plan("Big ch 6", "Big ch 6"),
        ];
        let left = skip_written(plans.clone(), 1);
        let files: Vec<&str> = left.iter().map(|p| p.file_name.as_str()).collect();
        assert_eq!(files, ["Big ch 5 (2)", "Big ch 6"]);
        assert_eq!(skip_written(plans.clone(), 0), plans);
        assert!(skip_written(plans, 9).is_empty());
    }
}
