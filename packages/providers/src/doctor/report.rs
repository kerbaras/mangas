//! What `providers check` found, and its table as plain text (pure).

use std::time::Duration;

/// The steps of a check, in the order they run; each needs the previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Status,
    Search,
    Manga,
    Chapters,
    Pages,
    Image,
}

pub const STEPS: [Step; 6] = [
    Step::Status,
    Step::Search,
    Step::Manga,
    Step::Chapters,
    Step::Pages,
    Step::Image,
];

impl Step {
    pub fn label(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Search => "search",
            Self::Manga => "manga",
            Self::Chapters => "chapters",
            Self::Pages => "pages",
            Self::Image => "image",
        }
    }
}

/// One step that ran: how long it took and what it found (`Ok`) or why it failed (`Err`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepReport {
    pub step: Step,
    pub elapsed: Duration,
    pub outcome: Result<String, String>,
}

/// The steps that ran, in order; the walk stops at the first failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub provider: String,
    pub steps: Vec<StepReport>,
}

impl Report {
    /// Every step ran and passed.
    pub fn passed(&self) -> bool {
        self.steps.len() == STEPS.len() && self.steps.iter().all(|s| s.outcome.is_ok())
    }

    pub fn elapsed(&self) -> Duration {
        self.steps.iter().map(|s| s.elapsed).sum()
    }

    pub fn failed_step(&self) -> Option<Step> {
        self.steps
            .iter()
            .find(|s| s.outcome.is_err())
            .map(|s| s.step)
    }
}

/// One line per step (steps after a failure show as not run), then the verdict.
pub fn lines(report: &Report) -> Vec<String> {
    let mut lines: Vec<String> = STEPS
        .iter()
        .map(|step| match report.steps.iter().find(|s| s.step == *step) {
            Some(s) => {
                let (mark, text) = match &s.outcome {
                    Ok(detail) => ("ok", detail.as_str()),
                    Err(reason) => ("FAIL", reason.as_str()),
                };
                row(step.label(), mark, &millis(s.elapsed), text)
            }
            None => row(step.label(), "-", "", "not run"),
        })
        .collect();
    lines.push(verdict(report));
    lines
}

fn row(label: &str, mark: &str, time: &str, text: &str) -> String {
    format!("{label:<9} {mark:<4} {time:>8}  {text}")
        .trim_end()
        .to_string()
}

fn verdict(report: &Report) -> String {
    let total = format!("{:.1} s", report.elapsed().as_secs_f64());
    match report.failed_step() {
        _ if report.passed() => format!("OK {}: {} steps in {total}", report.provider, STEPS.len()),
        Some(step) => format!("FAIL {}: {} failed", report.provider, step.label()),
        None => format!("FAIL {}: incomplete", report.provider),
    }
}

fn millis(elapsed: Duration) -> String {
    format!("{} ms", elapsed.as_millis())
}
