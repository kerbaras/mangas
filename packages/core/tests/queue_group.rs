//! The per-series group (#167) on queue rows and in the library's chapter counts.

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::download::queue::{enqueue, list};
use mangas_core::library::{
    list_entries, open_in_memory, set_chapter_group, upsert_chapters, upsert_manga, Library,
};

fn follow(lib: &Library, id: &str, releases: &[(&str, &str)]) -> Vec<Chapter> {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadot".into(),
        title: id.into(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    let chapters: Vec<Chapter> = releases
        .iter()
        .map(|(number, group)| Chapter {
            id: ChapterId(format!("{id}-{number}-{group}")),
            manga_id: manga.id.clone(),
            language: "en".into(),
            number: (*number).into(),
            group: (!group.is_empty()).then(|| (*group).into()),
            ..Default::default()
        })
        .collect();
    upsert_chapters(lib, "mangadot", &manga.id, "en", &chapters).unwrap();
    chapters
}

#[test]
fn queue_rows_carry_the_group_only_when_the_title_has_several() {
    let lib = open_in_memory().unwrap();
    let two = follow(&lib, "two", &[("4", "K Manga"), ("4", "Cutegurl Scan")]);
    let one = follow(&lib, "one", &[("1", "Solo")]);
    enqueue(&lib, "mangadot", [two, one].concat(), false).unwrap();
    let groups: Vec<(String, Option<String>)> = list(&lib)
        .unwrap()
        .into_iter()
        .map(|j| (j.manga_title, j.group))
        .collect();
    assert_eq!(
        groups,
        [
            ("two".to_string(), Some("K Manga".to_string())),
            ("two".to_string(), Some("Cutegurl Scan".to_string())),
            ("one".to_string(), None),
        ]
    );
}

#[test]
fn chapter_counts_follow_the_saved_group() {
    let lib = open_in_memory().unwrap();
    follow(
        &lib,
        "2098",
        &[("1", "A"), ("2", "A"), ("2", "B"), ("3", "B")],
    );
    let id = MangaId("2098".into());
    let counts = |lib: &Library| list_entries(lib).unwrap()[0].chapters;

    assert_eq!((counts(&lib).total, counts(&lib).unread), (3, 3));
    set_chapter_group(&lib, "mangadot", &id, Some("A")).unwrap();
    assert_eq!((counts(&lib).total, counts(&lib).unread), (2, 2));
    set_chapter_group(&lib, "mangadot", &id, None).unwrap();
    assert_eq!(counts(&lib).total, 3, "All counts every group");
}

#[test]
fn a_release_without_a_group_counts_as_a_group_of_its_own() {
    let lib = open_in_memory().unwrap();
    let mixed = follow(&lib, "mixed", &[("4", "K Manga"), ("4", "")]);
    enqueue(&lib, "mangadot", mixed, false).unwrap();
    let groups: Vec<Option<String>> = list(&lib).unwrap().into_iter().map(|j| j.group).collect();
    assert_eq!(groups, [Some("K Manga".to_string()), None]);
}

#[test]
fn the_saved_groups_release_of_a_known_chapter_is_new() {
    let lib = open_in_memory().unwrap();
    let id = MangaId("knight".into());
    follow(&lib, "knight", &[("1", "B")]);
    set_chapter_group(&lib, "mangadot", &id, Some("A")).unwrap();
    follow(&lib, "knight", &[("1", "B"), ("1", "A")]);
    assert_eq!(list_entries(&lib).unwrap()[0].chapters.new, 1);
}
