//! What a settling download run says when chapters are left undone.

use mangas_core::download::queue::{Job, JobState, Settled};

use super::watch::label;
use crate::exit::Exit;

/// Exit code of a run that ended with chapters queued or paused (never 0: they're not done).
pub const PAUSED_EXIT: u8 = 2;

const CONTINUE: &str = "`mangas queue resume --all` then `mangas queue run` downloads them";

/// When chapters are left queued or paused (the queue-wide pause or per-chapter pauses): lines
/// naming the chapters done and the ones left, and the error to exit with.
pub fn paused_report(settled: &Settled) -> Option<(Vec<String>, Exit)> {
    let left = |j: &&Job| matches!(j.state, JobState::Queued | JobState::Paused);
    let names = |keep: &dyn Fn(&&Job) -> bool| {
        let picked: Vec<String> = settled.jobs.iter().filter(keep).map(label).collect();
        picked.join(", ")
    };
    let waiting = settled.jobs.iter().filter(left).count();
    if waiting == 0 {
        return None;
    }
    let mut lines = Vec::new();
    let done = names(&|j| j.state == JobState::Done);
    if !done.is_empty() {
        lines.push(format!("done: {done}"));
    }
    lines.push(format!("still queued: {}", names(&left)));
    let why = match settled.paused {
        true => format!("the download queue is paused with {waiting} chapter(s) still queued"),
        false => format!("{waiting} chapter(s) paused"),
    };
    let message = format!("{why}; {CONTINUE}");
    Some((
        lines,
        Exit {
            code: PAUSED_EXIT,
            message,
        },
    ))
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
    fn chapters_paused_by_pause_all_or_one_by_one_still_exit_2() {
        let jobs = vec![job("1", JobState::Done), job("2", JobState::Paused)];
        let all = Settled {
            jobs: jobs.clone(),
            paused: true,
        };
        let (lines, exit) = paused_report(&all).unwrap();
        assert_eq!(lines, ["done: ch 1", "still queued: ch 2"]);
        assert!(
            exit.message.starts_with("the download queue is paused"),
            "{}",
            exit.message
        );
        let (_, exit) = paused_report(&Settled {
            jobs,
            paused: false,
        })
        .unwrap();
        assert_eq!(
            (exit.code, exit.message.starts_with("1 chapter(s) paused;")),
            (2, true)
        );
    }

    #[test]
    fn a_run_that_finished_reports_nothing() {
        let jobs = vec![job("1", JobState::Done), job("2", JobState::Failed)];
        assert_eq!(paused_report(&Settled { jobs, paused: true }), None);
    }
}
