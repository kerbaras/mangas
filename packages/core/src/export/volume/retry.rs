//! "Try again" after a run that stopped part-way: only the books it didn't write.

use super::VolumePlan;

/// The plans titled in `only`, in plan order, or every plan when `only` is `None`. A retry after
/// a stopped run names the failed book and the ones after it, so the books the run kept are not
/// written again as ` (2)` copies. Replacing or not stays the request's choice.
pub fn only_titles(plans: Vec<VolumePlan>, only: Option<&[String]>) -> Vec<VolumePlan> {
    match only {
        None => plans,
        Some(titles) => plans
            .into_iter()
            .filter(|p| titles.contains(&p.title))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(title: &str) -> VolumePlan {
        VolumePlan {
            number: None,
            title: title.into(),
            file_name: format!("{title}.epub"),
            chapters: Vec::new(),
        }
    }

    fn titles(plans: &[VolumePlan]) -> Vec<&str> {
        plans.iter().map(|p| p.title.as_str()).collect()
    }

    #[test]
    fn keeps_every_plan_without_a_list() {
        let plans = vec![plan("v1"), plan("v2")];
        assert_eq!(titles(&only_titles(plans, None)), ["v1", "v2"]);
    }

    #[test]
    fn keeps_only_the_listed_books_in_plan_order() {
        let plans = vec![plan("v1"), plan("v2"), plan("v3")];
        let only = ["v3".to_string(), "v2".to_string()];
        assert_eq!(titles(&only_titles(plans, Some(&only))), ["v2", "v3"]);
    }
}
