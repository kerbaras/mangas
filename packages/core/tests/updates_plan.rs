//! Pure parts of update checks: which titles, what is new, the summary line.

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::updates::{
    new_chapters, notification, select, summary, CheckFailure, Known, TitleUpdate, UpdatePolicy,
    UpdateReport,
};
use mangas_core::library::{ChapterCounts, LibraryEntry, ReadingStatus, LOCAL_SOURCE};

fn entry(source: &str, id: &str, status: ReadingStatus) -> LibraryEntry {
    LibraryEntry {
        manga: Manga {
            id: MangaId(id.into()),
            source: source.into(),
            title: id.into(),
            ..Default::default()
        },
        reading_status: status,
        categories: vec![],
        added_at: String::new(),
        updated_at: String::new(),
        chapters: ChapterCounts::default(),
        auto_download: id == "auto",
        chapter_group: None,
    }
}

fn chapter(id: &str, number: &str, language: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        number: number.into(),
        language: language.into(),
        ..Default::default()
    }
}

#[test]
fn finished_dropped_and_local_titles_are_skipped_by_default() {
    let entries = [
        entry("mangadex", "auto", ReadingStatus::Reading),
        entry("mangadex", "later", ReadingStatus::PlanToRead),
        entry("mangadex", "paused", ReadingStatus::OnHold),
        entry("mangadex", "done", ReadingStatus::Completed),
        entry("comick", "gone", ReadingStatus::Dropped),
        entry(LOCAL_SOURCE, "imported", ReadingStatus::Reading),
    ];
    let ids = |targets: &[mangas_core::library::updates::Target]| -> Vec<String> {
        targets.iter().map(|t| t.manga.id.0.clone()).collect()
    };
    let (targets, skipped) = select(&entries, &UpdatePolicy::default());
    assert_eq!(ids(&targets), ["auto", "later", "paused"]);
    assert_eq!(skipped, 3);
    assert!(targets[0].auto_download && !targets[1].auto_download);

    let all = UpdatePolicy {
        include_finished: true,
        ..Default::default()
    };
    let (targets, skipped) = select(&entries, &all);
    assert_eq!(ids(&targets), ["auto", "later", "paused", "done", "gone"]);
    assert_eq!(skipped, 1, "local imports have no source to ask");

    let comick = UpdatePolicy {
        source: Some("comick".into()),
        include_finished: true,
    };
    let (targets, skipped) = select(&entries, &comick);
    assert_eq!((ids(&targets), skipped), (vec!["gone".to_string()], 5));
}

#[test]
fn new_means_unseen_id_in_a_language_synced_before() {
    let known = Known {
        chapter_ids: ["a", "b"].map(String::from).into(),
        languages: ["en".to_string()].into(),
        ..Known::default()
    };
    let lists = vec![
        (
            "en".to_string(),
            vec![chapter("a", "1", "en"), chapter("c", "3", "en")],
        ),
        // Never synced in Spanish: its whole list is the baseline, not news.
        ("es".to_string(), vec![chapter("x", "3", "es")]),
    ];
    let new: Vec<String> = new_chapters(&known, &lists)
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(new, ["c"]);
    assert!(new_chapters(&Known::default(), &lists).is_empty());
}

fn update(title: &str, chapters: Vec<Chapter>) -> TitleUpdate {
    TitleUpdate {
        source: "mangadex".into(),
        manga_id: MangaId(title.into()),
        title: title.into(),
        auto_download: false,
        chapter_group: None,
        chapters,
    }
}

fn failure(source: &str, title: &str) -> CheckFailure {
    CheckFailure {
        source: source.into(),
        manga_id: MangaId(title.into()),
        title: title.into(),
        error: "network: timed out".into(),
        challenge: false,
    }
}

#[test]
fn summary_counts_numbers_once_and_providers_once() {
    let mut report = UpdateReport::default();
    assert_eq!(summary(&report).line(), "Up to date");

    report.updates = vec![
        update(
            "A",
            vec![chapter("1", "12", "en"), chapter("2", "12.0", "es")],
        ),
        update("B", vec![chapter("3", "5", "en"), chapter("4", "6", "en")]),
    ];
    let s = summary(&report);
    assert_eq!((s.chapters, s.titles, s.unreachable), (3, 2, 0));
    assert_eq!(s.line(), "3 new chapters in 2 titles");

    report.failures = vec![
        failure("comick", "C"),
        failure("comick", "D"),
        failure("weebcentral", "E"),
    ];
    assert_eq!(
        summary(&report).line(),
        "3 new chapters in 2 titles · 2 providers unreachable"
    );

    report.updates = vec![update("A", vec![chapter("1", "1", "en")])];
    report.failures.truncate(1);
    assert_eq!(
        summary(&report).line(),
        "1 new chapter in 1 title · 1 provider unreachable"
    );
    report.updates.clear();
    assert_eq!(summary(&report).line(), "1 provider unreachable");
}

#[test]
fn one_notification_per_run_naming_a_few_titles() {
    let mut report = UpdateReport {
        failures: vec![failure("comick", "C")],
        ..Default::default()
    };
    assert_eq!(notification(&report), None, "failures alone don't notify");
    report.updates = ["A", "B", "C", "D", "E"]
        .iter()
        .map(|t| update(t, vec![chapter(t, "1", "en")]))
        .collect();
    let (title, body) = notification(&report).unwrap();
    assert_eq!(title, "5 new chapters in 5 titles");
    assert_eq!(body, "A, B, C and 2 more");
    report.updates.truncate(2);
    assert_eq!(notification(&report).unwrap().1, "A, B");
}

#[test]
fn another_groups_release_of_a_stored_number_is_not_new() {
    let known = Known {
        chapter_ids: ["a"].map(String::from).into(),
        languages: ["en".to_string()].into(),
        numbers: [("en".to_string(), "1".to_string())].into(),
    };
    let lists = vec![(
        "en".to_string(),
        vec![
            chapter("a", "1", "en"),
            chapter("koi-1", "1", "en"),
            chapter("b", "2", "en"),
        ],
    )];
    let new: Vec<String> = new_chapters(&known, &lists)
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(new, ["b"]);
}
