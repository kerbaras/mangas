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

#[test]
fn collapse_keeps_one_release_per_number_and_language() {
    use mangas_core::domain::{collapse_groups, Availability};
    let release = |id: &str, number: &str, lang: &str, group: &str| Chapter {
        id: ChapterId(id.into()),
        number: number.into(),
        language: lang.into(),
        group: Some(group.into()),
        ..Default::default()
    };
    let mut external = release("ext-1", "1", "en", "Koi");
    external.availability = Availability::External { url: None };
    let list = vec![
        external,
        release("r-1", "1", "en", "r/OPM"),
        release("koi-2", "2", "en", "Koi"),
        release("r-2", "2", "en", "r/OPM"),
        release("es-2", "2", "es", "Koi"),
        release("x", "", "en", "Koi"),
    ];
    let ids = |l: Vec<Chapter>| -> Vec<String> { l.into_iter().map(|c| c.id.0).collect() };
    assert_eq!(
        ids(collapse_groups(list.clone(), &[])),
        ["r-1", "koi-2", "es-2", "x"]
    );
    assert_eq!(
        ids(collapse_groups(list, &["R/opm".into()])),
        ["r-1", "r-2", "es-2", "x"]
    );
}

#[test]
fn collapse_keeps_chapters_of_restarted_volumes() {
    use mangas_core::domain::collapse_groups;
    let release = |id: &str, volume: Option<&str>, group: &str| Chapter {
        id: ChapterId(id.into()),
        number: "1".into(),
        language: "en".into(),
        volume: volume.map(String::from),
        group: Some(group.into()),
        ..Default::default()
    };
    let list = vec![
        release("v1", Some("1"), "A"),
        release("v1-b", None, "B"),
        release("v2", Some("2"), "A"),
    ];
    let ids: Vec<String> = collapse_groups(list, &[])
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(ids, ["v1", "v2"]);
}

#[test]
fn a_release_without_volume_never_bridges_two_volumes() {
    use mangas_core::domain::collapse_groups;
    let release = |id: &str, volume: Option<&str>, group: &str| Chapter {
        id: ChapterId(id.into()),
        number: "1".into(),
        language: "en".into(),
        volume: volume.map(String::from),
        group: Some(group.into()),
        ..Default::default()
    };
    let list = vec![
        release("none-b", None, "B"),
        release("v1", Some("1"), "A"),
        release("v2", Some("2"), "A"),
    ];
    let ids: Vec<String> = collapse_groups(list, &["B".into()])
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(ids, ["none-b", "v2"]);
}

#[test]
fn collapse_keeps_the_release_already_on_disk() {
    use mangas_core::domain::collapse_groups;
    let release = |id: &str, group: &str| Chapter {
        id: ChapterId(id.into()),
        number: "157".into(),
        language: "en".into(),
        group: Some(group.into()),
        ..Default::default()
    };
    let mut koi = release("koi", "The Koi pond.");
    koi.local_path = Some("/dl/157".into());
    let list = vec![release("r", "r/OnePunchMan"), koi];
    let kept = collapse_groups(list, &["r/OnePunchMan".into()]);
    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept[0].id.0, "koi",
        "a downloaded release beats preferred groups"
    );
}
