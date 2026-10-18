//! What a settling download run says when chapters are left undone.

use mangas_core::download::queue::{Job, JobState, Settled};

use super::watch::{count, label};
use crate::exit::Exit;

/// Exit code of a run the queue-wide pause stopped with chapters still queued.
pub const PAUSED_EXIT: u8 = 2;

const CONTINUE: &str = "`mangas queue resume --all` then `mangas queue run` downloads them";

/// When the user's queue-wide pause stopped the run: lines naming the chapters done and the
/// ones still queued, and the error to exit with.
pub fn paused_report(settled: &Settled) -> Option<(Vec<String>, Exit)> {
    if !settled.paused {
        return None;
    }
    let names = |state| {
        let picked: Vec<String> = settled
            .jobs
            .iter()
            .filter(|j| j.state == state)
            .map(label)
            .collect();
        picked.join(", ")
    };
    let mut lines = Vec::new();
    let done = names(JobState::Done);
    if !done.is_empty() {
        lines.push(format!("done: {done}"));
    }
    lines.push(format!("still queued: {}", names(JobState::Queued)));
    let queued = count(&settled.jobs, JobState::Queued);
    let message =
        format!("the download queue is paused with {queued} chapter(s) still queued; {CONTINUE}");
    Some((
        lines,
        Exit {
            code: PAUSED_EXIT,
            message,
        },
    ))
}

/// Chapters paused one by one (the queue itself still runs).
pub fn paused_note(jobs: &[Job]) -> Option<String> {
    match count(jobs, JobState::Paused) {
        0 => None,
        n => Some(format!("{n} chapter(s) paused; {CONTINUE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(number: &str, state: JobState) -> Job {
        Job {
            state,
            number: number.into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_paused_queue_names_what_finished_and_what_waits_and_exits_2() {
        let jobs = vec![job("1", JobState::Done), job("2", JobState::Queued)];
        let (lines, exit) = paused_report(&Settled { jobs, paused: true }).unwrap();
        assert_eq!(lines, ["done: ch 1", "still queued: ch 2"]);
        assert_eq!(exit.code, 2);
        assert!(exit
            .message
            .starts_with("the download queue is paused with 1 chapter(s)"));
        assert!(
            exit.message.contains("`mangas queue resume --all`"),
            "{}",
            exit.message
        );
    }

    #[test]
    fn a_pause_before_the_start_lists_every_chapter_as_queued() {
        let jobs = vec![job("7", JobState::Queued), job("8", JobState::Queued)];
        let (lines, _) = paused_report(&Settled { jobs, paused: true }).unwrap();
        assert_eq!(lines, ["still queued: ch 7, ch 8"]);
    }

    #[test]
    fn a_run_that_finished_reports_nothing() {
        let jobs = vec![job("1", JobState::Done)];
        assert_eq!(
            paused_report(&Settled {
                jobs: jobs.clone(),
                paused: false
            }),
            None
        );
        assert_eq!(paused_note(&jobs), None);
        assert!(paused_note(&[job("3", JobState::Paused)]).is_some());
    }
}
