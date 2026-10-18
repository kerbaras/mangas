//! `providers check` over several languages: the first one whose walk reaches chapters.

use std::future::Future;

use super::{Report, Step};

/// Runs `walk(language)` for each language in order and keeps the first report that found
/// chapters, noting the languages skipped before it. A status failure is the same in every
/// language, so it stops at once. When every language fails, the last report is returned with
/// every language's reason.
pub async fn first_language<W, Fut, E>(languages: &[String], walk: W) -> Result<Report, E>
where
    W: Fn(String) -> Fut,
    Fut: Future<Output = Result<Report, E>>,
{
    let mut skipped = Vec::new();
    let mut last = None;
    for language in languages {
        let report = walk(language.clone()).await?;
        if found_chapters(&report) || status_failed(&report) {
            return Ok(with_skipped(report, &skipped));
        }
        skipped.push(format!("{language}: {}", failure(&report)));
        last = Some(report);
    }
    Ok(match last {
        Some(report) if skipped.len() > 1 => all_failed(report, &skipped),
        Some(report) => report,
        None => Report {
            provider: String::new(),
            steps: Vec::new(),
        },
    })
}

fn found_chapters(report: &Report) -> bool {
    report
        .steps
        .iter()
        .any(|s| s.step == Step::Chapters && s.outcome.is_ok())
}

fn status_failed(report: &Report) -> bool {
    report
        .steps
        .first()
        .is_some_and(|s| s.step == Step::Status && s.outcome.is_err())
}

fn failure(report: &Report) -> String {
    report
        .steps
        .iter()
        .find_map(|s| s.outcome.as_ref().err().cloned())
        .unwrap_or_default()
}

fn with_skipped(mut report: Report, skipped: &[String]) -> Report {
    if skipped.is_empty() {
        return report;
    }
    for step in report.steps.iter_mut().filter(|s| s.step == Step::Chapters) {
        if let Ok(detail) = &step.outcome {
            step.outcome = Ok(format!("{detail}; skipped {}", skipped.join("; ")));
        }
    }
    report
}

fn all_failed(mut report: Report, skipped: &[String]) -> Report {
    if let Some(step) = report.steps.iter_mut().find(|s| s.outcome.is_err()) {
        let what = match step.step {
            Step::Chapters => "no language had chapters",
            _ => "every language failed",
        };
        step.outcome = Err(format!("{what} ({})", skipped.join("; ")));
    }
    report
}
