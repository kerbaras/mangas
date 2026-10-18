//! Auto-download hands new chapters to the persistent queue: one per number, preferred
//! language first, only for titles that ask for it unless everything is requested.

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::download::queue;
use mangas_core::library::updates::{enqueue_new, TitleUpdate, UpdateReport};
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
