//! What a run of several books leaves behind when one of them fails. The rule: the run stops at
//! the failing book; the books written before it are complete and stay (they are recorded as
//! exported); the failing book leaves nothing (each file is written to a temporary name and
//! renamed only once whole, see `export::output`); the books after it are not started. The
//! outcome reports all three so the CLI and the app can say what is on disk. The kept books
//! are complete, so they are sent like the books of a run that finished.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::unwritten::unwritten_paths;
use crate::domain::Result;
use crate::export::kindle::{ExportRequest, ExportStats};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeOutcome {
    /// Books written, complete, in order.
    pub written: Vec<ExportStats>,
    /// The book that failed, if any: nothing of it is on disk.
    pub failed: Option<FailedVolume>,
    /// Books after the failed one, not started.
    pub not_written: Vec<PlannedVolume>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedVolume {
    pub title: String,
    pub output: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedVolume {
    pub title: String,
    pub output: PathBuf,
}

/// Writes each request in order with `write` (its index and request), stopping at the first
/// failure as described in the module docs.
pub fn export_each(
    reqs: &[ExportRequest],
    mut write: impl FnMut(usize, &ExportRequest) -> Result<ExportStats>,
) -> VolumeOutcome {
    let mut written = Vec::with_capacity(reqs.len());
    for (index, req) in reqs.iter().enumerate() {
        match write(index, req) {
            Ok(stats) => written.push(stats),
            Err(e) => return VolumeOutcome::stopped(reqs, index, written, e.to_string()),
        }
    }
    VolumeOutcome {
        written,
        failed: None,
        not_written: Vec::new(),
    }
}

impl VolumeOutcome {
    /// The outcome of a run that wrote `written` and then failed on `reqs[index]`.
    pub fn stopped(
        reqs: &[ExportRequest],
        index: usize,
        written: Vec<ExportStats>,
        reason: String,
    ) -> Self {
        let rest = reqs.get(index..).unwrap_or_default();
        let mut unwritten = rest.iter().zip(unwritten_paths(rest));
        let failed = unwritten.next().map(|(r, output)| FailedVolume {
            title: r.title.clone(),
            output,
            reason,
        });
        let not_written = unwritten
            .map(|(r, output)| PlannedVolume {
                title: r.title.clone(),
                output,
            })
            .collect();
        VolumeOutcome {
            written,
            failed,
            not_written,
        }
    }

    /// One line for a run that stopped: what failed, what was kept and what wasn't started.
    pub fn summary(&self) -> Option<String> {
        let failed = self.failed.as_ref()?;
        let mut line = format!("{} failed: {}", failed.title, failed.reason);
        let kept = self.written.len();
        line.push_str(&match kept {
            0 => "; no book was written".to_string(),
            1 => "; the 1 book written before it is complete and kept".to_string(),
            n => format!("; the {n} books written before it are complete and kept"),
        });
        if !self.not_written.is_empty() {
            let titles: Vec<&str> = self.not_written.iter().map(|p| p.title.as_str()).collect();
            line.push_str(&format!("; not written: {}", titles.join(", ")));
        }
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Error;

    fn req(title: &str) -> ExportRequest {
        ExportRequest {
            chapter_dirs: Vec::new(),
            device: crate::export::profile("kindle-paperwhite12").unwrap(),
            format: crate::export::Format::Epub,
            cut: crate::smartcut::Preset::Balanced.options(),
            page: Default::default(),
            title: title.into(),
            title_explicit: false,
            output: PathBuf::from(format!("/k/{title}.epub")),
            replace: false,
            split: None,
            meta: Default::default(),
            labels: Default::default(),
        }
    }

    fn stats(r: &ExportRequest) -> ExportStats {
        ExportStats {
            pages: 1,
            strips: 1,
            source_rows: 1,
            pagination_ms: 0,
            filters_ms: 0,
            skipped: 0,
            empty_chapters: Vec::new(),
            output: r.output.clone(),
            detectors_error: None,
            parts: Vec::new(),
        }
    }

    #[test]
    fn stops_at_the_failing_book_and_reports_kept_failed_and_not_started() {
        let reqs = [req("v1"), req("v2"), req("v3"), req("v4")];
        let mut tried = Vec::new();
        let out = export_each(&reqs, |i, r| {
            tried.push(i);
            match i {
                2 => Err(Error::Other("disk full".into())),
                _ => Ok(stats(r)),
            }
        });
        assert_eq!(tried, [0, 1, 2]);
        assert_eq!(out.written.len(), 2);
        assert_eq!(
            out.failed.as_ref().unwrap().output,
            PathBuf::from("/k/v3.epub")
        );
        assert_eq!(out.not_written[0].title, "v4");
        assert_eq!(
            out.summary().unwrap(),
            "v3 failed: disk full; the 2 books written before it are complete and kept; \
             not written: v4"
        );
    }

    #[test]
    fn a_run_without_failure_has_no_summary() {
        let reqs = [req("v1")];
        let out = export_each(&reqs, |_, r| Ok(stats(r)));
        assert_eq!(
            (out.written.len(), out.failed.clone(), out.summary()),
            (1, None, None)
        );
    }
}
