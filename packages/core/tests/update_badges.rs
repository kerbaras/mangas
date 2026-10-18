//! "New" chapters: stamped on sync in languages synced before, counted until the title is opened.

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    list_entries, mark_viewed, open_in_memory, set_auto_download, upsert_chapters, upsert_manga,
    Library,
};

fn follow(lib: &Library, id: &str) -> MangaId {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: id.into(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    manga.id
}

fn chapters(manga: &MangaId, numbers: &[&str], language: &str) -> Vec<Chapter> {
    numbers
        .iter()
        .map(|n| Chapter {
            id: ChapterId(format!("{language}-{n}")),
            manga_id: manga.clone(),
            language: language.into(),
            number: (*n).into(),
            ..Default::default()
        })
        .collect()
}

/// Times have millisecond precision: keep "viewed" and "first seen" apart.
fn tick() {
    std::thread::sleep(std::time::Duration::from_millis(5));
}

fn new_count(lib: &Library) -> usize {
    list_entries(lib).unwrap()[0].chapters.new
}

#[test]
fn first_sync_is_the_baseline_later_chapters_are_new_until_viewed() {
    let lib = open_in_memory().unwrap();
    let id = follow(&lib, "a");
    upsert_chapters(
        &lib,
        "mangadex",
        &id,
        "en",
        &chapters(&id, &["1", "2"], "en"),
    )
    .unwrap();
    assert_eq!(new_count(&lib), 0, "following a title badges nothing");

    let more = chapters(&id, &["1", "2", "3", "4"], "en");
    upsert_chapters(&lib, "mangadex", &id, "en", &more).unwrap();
    assert_eq!(new_count(&lib), 2);

    // A language added later starts with its own baseline: no flood of "new" chapters.
    let es = chapters(&id, &["1", "2", "3", "4", "5"], "es");
    upsert_chapters(&lib, "mangadex", &id, "es", &es).unwrap();
    assert_eq!(new_count(&lib), 2);

    // Re-syncing what is already stored keeps the badge, opening the title clears it.
    upsert_chapters(&lib, "mangadex", &id, "en", &more).unwrap();
    assert_eq!(new_count(&lib), 2);
    tick();
    assert!(mark_viewed(&lib, "mangadex", &id).unwrap());
    assert_eq!(new_count(&lib), 0);
    tick();

    let next = chapters(&id, &["1", "2", "3", "4", "6"], "es");
    upsert_chapters(&lib, "mangadex", &id, "es", &next).unwrap();
    assert_eq!(new_count(&lib), 1);
    assert!(!mark_viewed(&lib, "mangadex", &MangaId("nope".into())).unwrap());
}

#[test]
fn auto_download_is_a_persisted_per_title_flag() {
    let lib = open_in_memory().unwrap();
    let id = follow(&lib, "a");
    assert!(!list_entries(&lib).unwrap()[0].auto_download);
    assert!(set_auto_download(&lib, "mangadex", &id, true).unwrap());
    assert!(list_entries(&lib).unwrap()[0].auto_download);
    upsert_manga(&lib, &list_entries(&lib).unwrap()[0].manga).unwrap();
    assert!(
        list_entries(&lib).unwrap()[0].auto_download,
        "a refresh keeps it"
    );
    assert!(set_auto_download(&lib, "mangadex", &id, false).unwrap());
    assert!(!list_entries(&lib).unwrap()[0].auto_download);
    assert!(!set_auto_download(&lib, "mangadex", &MangaId("x".into()), true).unwrap());
}
