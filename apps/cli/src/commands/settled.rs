//! What a settling download run says when chapters are left undone.

use mangas_core::download::queue::{is_final_error, Job, JobState, Settled};

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

/// The note for failed `noun`s. `mangas queue retry --all` is only offered for the ones a retry
/// can fix: a refused host or a chapter gone from the source fails the same way again.
pub fn failed_note(jobs: &[Job], noun: &str) -> Option<String> {
    let failed: Vec<&Job> = jobs
        .iter()
        .filter(|j| j.state == JobState::Failed)
        .collect();
    let fixed = |j: &&&Job| j.error.as_deref().is_some_and(is_final_error);
    let (total, stuck) = (failed.len(), failed.iter().filter(fixed).count());
    let mut note = match total {
        0 => return None,
        n => format!("{n} {noun}(s) failed"),
    };
    if stuck > 0 {
        note +=
            &format!("; {stuck} can't succeed on a retry (refused host or gone from the source)");
    }
    match total - stuck {
        0 => {}
        n if stuck > 0 => {
            note += &format!("; retry the other {n} one at a time with `mangas queue retry <id>`")
        }
        _ => note += "; `mangas queue retry --all` tries them again",
    }
    Some(note)
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

    fn failed(error: String) -> Job {
        Job {
            error: Some(error),
            ..job("1", JobState::Failed)
        }
    }

    fn refused() -> String {
        use mangas_core::domain::Error;
        use mangas_core::download::queue::classify;
        let ip = "10.0.0.1".parse().unwrap();
        classify(&Error::BlockedAddress {
            host: "cdn".into(),
            ip,
        })
        .message
    }

    #[test]
    fn a_refused_download_is_not_offered_a_retry() {
        let note = failed_note(&[failed(refused())], "chapter").unwrap();
        assert!(!note.contains("retry --all"), "{note}");
        assert!(note.contains("1 can't succeed on a retry"), "{note}");
    }

    #[test]
    fn retry_is_offered_for_the_failures_it_can_fix() {
        let jobs = [failed(refused()), failed("network: timed out".into())];
        let note = failed_note(&jobs, "download").unwrap();
        assert!(note.starts_with("2 download(s) failed"), "{note}");
        assert!(note.ends_with("retry the other 1 one at a time with `mangas queue retry <id>`"));
        assert!(!note.contains("--all"), "{note}");
        let note = failed_note(&jobs[1..], "chapter").unwrap();
        assert_eq!(
            note,
            "1 chapter(s) failed; `mangas queue retry --all` tries them again"
        );
        assert_eq!(failed_note(&[job("1", JobState::Done)], "chapter"), None);
    }
}
