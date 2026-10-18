//! Auto-download hands new chapters to the persistent queue: one per number, preferred
//! language first, only for titles that ask for it unless everything is requested.

use mangas_core::domain::{Availability, Chapter, ChapterId, Manga, MangaId};
use mangas_core::download::queue;
use mangas_core::library::updates::{enqueue_new, to_download, TitleUpdate, UpdateReport};
use mangas_core::library::{
    open_in_memory, set_auto_download, upsert_chapters, upsert_manga, Library,
};

fn follow(lib: &Library, id: &str, langs: &[(&str, &[&str])]) -> Vec<Chapter> {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: id.into(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    let mut all = Vec::new();
    for (language, numbers) in langs {
        let list: Vec<Chapter> = numbers
            .iter()
            .map(|n| Chapter {
                id: ChapterId(format!("{id}-{language}-{n}")),
                manga_id: manga.id.clone(),
                language: (*language).into(),
                number: (*n).into(),
                ..Default::default()
            })
            .collect();
        upsert_chapters(lib, "mangadex", &manga.id, language, &list).unwrap();
        all.extend(list);
    }
    all
}

fn update(id: &str, auto_download: bool, chapters: Vec<Chapter>) -> TitleUpdate {
    TitleUpdate {
        source: "mangadex".into(),
        manga_id: MangaId(id.into()),
        title: id.into(),
        auto_download,
        chapter_group: None,
        chapters,
    }
}

#[test]
fn queues_auto_download_titles_once_per_number_in_the_preferred_language() {
    let lib = open_in_memory().unwrap();
    let a = follow(&lib, "a", &[("es", &["5"]), ("en", &["5", "6"])]);
    let b = follow(&lib, "b", &[("en", &["9"])]);
    set_auto_download(&lib, "mangadex", &MangaId("a".into()), true).unwrap();
    let report = UpdateReport {
        updates: vec![update("a", true, a), update("b", false, b)],
        ..Default::default()
    };

    let jobs = enqueue_new(&lib, &report, false).unwrap();
    let queued: Vec<String> = queue::list(&lib)
        .unwrap()
        .into_iter()
        .filter(|j| jobs.contains(&j.id))
        .map(|j| j.chapter_id.0)
        .collect();
    assert_eq!(queued, ["a-es-5", "a-en-6"]);

    let all = enqueue_new(&lib, &report, true).unwrap();
    assert_eq!(
        all.len(),
        3,
        "everything adds b; a's jobs are already on their way"
    );
    assert!(jobs.iter().all(|j| all.contains(j)));
}

#[test]
fn a_locked_preferred_release_falls_back_to_one_that_can_be_downloaded() {
    let chapter = |id: &str, number: &str, availability| Chapter {
        id: ChapterId(id.into()),
        number: number.into(),
        availability,
        ..Default::default()
    };
    let chapters = vec![
        chapter("es-5", "5", Availability::Locked),
        chapter("en-5", "5", Availability::Available),
        chapter("es-6", "6", Availability::Locked),
    ];
    let ids: Vec<String> = to_download(&update("a", true, chapters), &[])
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(
        ids,
        ["en-5", "es-6"],
        "6 has no other release; the queue reports it locked"
    );
}

#[test]
fn auto_download_takes_only_the_titles_group() {
    let grouped = |id: &str, number: &str, group: &str| Chapter {
        id: ChapterId(id.into()),
        number: number.into(),
        language: "en".into(),
        group: Some(group.into()),
        ..Default::default()
    };
    let chapters = vec![
        grouped("mirror-7", "7", "MangaDot mirror"),
        grouped("k-7", "7", "K Manga"),
        grouped("mirror-8", "8", "MangaDot mirror"),
        grouped("k-9", "9", "K Manga"),
    ];
    let ids = |u: &TitleUpdate| -> Vec<String> {
        to_download(u, &[]).into_iter().map(|c| c.id.0).collect()
    };
    let mut narrowed = update("a", true, chapters.clone());
    narrowed.chapter_group = Some("K Manga".into());
    assert_eq!(ids(&narrowed), ["k-7", "k-9"]);
    assert_eq!(
        ids(&update("a", true, chapters.clone())),
        ["mirror-7", "mirror-8", "k-9"]
    );
    let preferred = to_download(&update("a", true, chapters), &["k manga".into()]);
    let preferred: Vec<String> = preferred.into_iter().map(|c| c.id.0).collect();
    assert_eq!(
        preferred,
        ["k-7", "mirror-8", "k-9"],
        "no group saved: preferred groups win"
    );
}
