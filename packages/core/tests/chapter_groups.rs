//! Per-series group filter (#160): counting groups, narrowing a list, and the saved choice.

use mangas_core::domain::{groups_of, in_group, Chapter, ChapterId, GroupCount, Manga, MangaId};
use mangas_core::library::{
    chapter_group, list_entries, open_in_memory, set_chapter_group, upsert_manga,
};

fn chapter(id: &str, group: Option<&str>) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        number: "1".into(),
        group: group.map(String::from),
        ..Default::default()
    }
}

fn list() -> Vec<Chapter> {
    vec![
        chapter("1", Some("Official")),
        chapter("2", Some("K Manga")),
        chapter("3", Some("K Manga")),
        chapter("4", None),
        chapter("5", Some("A, B")),
    ]
}

#[test]
fn groups_are_counted_most_chapters_first() {
    let count = |name: &str, chapters| GroupCount {
        name: name.into(),
        chapters,
    };
    assert_eq!(
        groups_of(&list()),
        [count("K Manga", 2), count("A, B", 1), count("Official", 1)]
    );
}

#[test]
fn narrowing_keeps_only_the_whole_label() {
    let ids = |g| -> Vec<String> { in_group(list(), g).into_iter().map(|c| c.id.0).collect() };
    assert_eq!(ids(Some("K Manga")), ["2", "3"]);
    assert_eq!(ids(Some("A")), Vec::<String>::new());
    assert_eq!(ids(None).len(), 5);
}

#[test]
fn the_choice_is_saved_per_title_and_cleared_by_all() {
    let lib = open_in_memory().unwrap();
    let id = MangaId("2098".into());
    let manga = Manga {
        id: id.clone(),
        source: "mangadot".into(),
        title: "Knight".into(),
        ..Default::default()
    };
    upsert_manga(&lib, &manga).unwrap();
    assert_eq!(chapter_group(&lib, "mangadot", &id).unwrap(), None);

    assert!(set_chapter_group(&lib, "mangadot", &id, Some(" K Manga ")).unwrap());
    assert_eq!(
        chapter_group(&lib, "mangadot", &id).unwrap().as_deref(),
        Some("K Manga")
    );
    let entries = list_entries(&lib).unwrap();
    assert_eq!(entries[0].chapter_group.as_deref(), Some("K Manga"));

    assert!(set_chapter_group(&lib, "mangadot", &id, Some("  ")).unwrap());
    assert_eq!(chapter_group(&lib, "mangadot", &id).unwrap(), None);
    let other = MangaId("9".into());
    assert!(!set_chapter_group(&lib, "mangadot", &other, Some("X")).unwrap());
}
