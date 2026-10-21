//! "Try again" through the real planner: same-titled per-chapter books, Replace on.

use mangas_core::domain::{Chapter, ChapterId, Manga};
use mangas_core::export::volume::{self, Grouping, KeptBook};

fn chapter(id: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        number: "1".into(),
        language: "en".into(),
        local_path: Some(format!("/tmp/{id}")),
        ..Default::default()
    }
}

fn names(plans: Vec<volume::VolumePlan>, dir: &std::path::Path) -> Vec<String> {
    let named = volume::named_as_written(plans, dir, "epub", true).unwrap();
    named.into_iter().map(|p| p.file_name).collect()
}

#[test]
fn a_replacing_retry_never_writes_over_the_kept_book() {
    let dir = tempfile::tempdir().unwrap();
    let manga = Manga {
        title: "X".into(),
        ..Default::default()
    };
    let plan = |ids: &[&str]| {
        let chapters: Vec<Chapter> = ids.iter().map(|id| chapter(id)).collect();
        volume::plan(&manga, &chapters, Grouping::PerChapter, None, None)
    };
    // First run: b1 and c1, same title; it kept b1's book and failed on c1.
    let first = names(
        volume::retry_plans(plan(&["b1", "c1"]), &[], "epub", true),
        dir.path(),
    );
    assert_eq!(first[1], first[0].replace(".epub", " (2).epub"));
    let kept = [KeptBook {
        chapters: vec!["b1".into()],
        file: first[0].clone(),
    }];
    // a1 finished downloading since and plans ahead of b1.
    let retry = volume::retry_plans(plan(&["a1", "b1", "c1"]), &kept, "epub", true);
    let ids: Vec<_> = retry.iter().map(|p| p.chapters[0].id.0.clone()).collect();
    assert_eq!(ids, ["a1", "c1"]);
    let written = names(retry, dir.path());
    assert!(
        !written.contains(&first[0]),
        "{written:?} writes over the kept {}",
        first[0]
    );
}
