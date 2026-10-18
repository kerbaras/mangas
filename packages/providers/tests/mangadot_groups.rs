//! MangaDot lists one row per group for the same chapter (#160): the mapping credits each one.
//! The fixture is frozen from the live `/api/manga/2098/chapters/list` (chapters 1, 1.2 and 2).

use std::collections::BTreeSet;

use mangas_core::domain::MangaId;
use mangas_providers::mangadot::{parse_chapters, MIRROR_GROUP};

const DUPLICATES: &str = include_str!("fixtures/mangadot/chapters_duplicates.json");

fn groups_of(number: &str) -> Vec<String> {
    parse_chapters(DUPLICATES, &MangaId("2098".into()), "en")
        .unwrap()
        .into_iter()
        .filter(|c| c.number == number)
        .map(|c| c.group.unwrap_or_default())
        .collect()
}

#[test]
fn every_duplicate_row_names_a_distinct_group() {
    let ch1 = groups_of("1");
    assert_eq!(ch1.len(), 5);
    let distinct: BTreeSet<&String> = ch1.iter().collect();
    assert_eq!(distinct.len(), ch1.len(), "{ch1:?}");
    for name in [
        "K Manga",
        "Official",
        "ThePervertedScanner",
        "Cutegurl Scan",
    ] {
        assert!(ch1.iter().any(|g| g == name), "{name} missing from {ch1:?}");
    }
    assert!(ch1.iter().any(|g| g == MIRROR_GROUP));
    assert_eq!(groups_of("1.2").len(), 2);
}

#[test]
fn mirrored_chapters_get_the_mirror_label_not_official() {
    let chapters = parse_chapters(DUPLICATES, &MangaId("2098".into()), "en").unwrap();
    let mirrored: Vec<_> = chapters
        .iter()
        .filter(|c| c.id.0.starts_with("c:"))
        .collect();
    assert!(!mirrored.is_empty());
    assert!(mirrored
        .iter()
        .all(|c| c.group.as_deref() == Some(MIRROR_GROUP)));
    let official = chapters
        .iter()
        .find(|c| c.group.as_deref() == Some("Official"));
    assert!(official.is_some_and(|c| c.id.0.starts_with("u:")));
}

#[test]
fn group_falls_back_from_groups_to_group_name_to_scanlator() {
    let body = r#"[
     {"id":1,"chapter_number":1,"language":"en","source":"user","groups":[{"name":"A"},{"name":" B "},{"name":"A"}],"group_name":"X","scanlator_name":"Y"},
     {"id":2,"chapter_number":2,"language":"en","source":"user","groups":[],"group_name":"No Group","scanlator_name":"No-group"},
     {"id":3,"chapter_number":3,"language":"en","source":"user","group_name":"  ","scanlator_name":"Solo"},
     {"id":4,"chapter_number":4,"language":"en","source":"user"}
    ]"#;
    let groups: Vec<_> = parse_chapters(body, &MangaId("1".into()), "en")
        .unwrap()
        .into_iter()
        .map(|c| c.group)
        .collect();
    let want = ["A, B", "No Group", "Solo", MIRROR_GROUP];
    assert_eq!(groups, want.map(|g| Some(g.to_string())));
}
