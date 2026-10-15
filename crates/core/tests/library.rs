use std::path::Path;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, MangaStatus};
use mangas_core::library::{
    get_manga, list_chapters, list_manga, mark_downloaded, open, open_in_memory, remove_manga,
    upsert_chapters, upsert_manga,
};

fn sample(id: &str, title: &str) -> Manga {
    Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: title.into(),
        description: String::new(),
        cover_url: None,
        status: MangaStatus::Ongoing,
    }
}

fn chapter(manga: &str, id: &str, number: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        manga_id: MangaId(manga.into()),
        title: Some(format!("Chapter {number}")),
        language: "en".into(),
        volume: None,
        number: number.into(),
        local_path: None,
    }
}

#[test]
fn upsert_then_list_roundtrips() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("a", "Alpha")).unwrap();
    upsert_manga(&lib, &sample("a", "Alpha (renamed)")).unwrap();
    upsert_manga(&lib, &sample("b", "Beta")).unwrap();
    let all = list_manga(&lib).unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|m| m.title == "Alpha (renamed)"));
    assert_eq!(
        get_manga(&lib, "mangadex", &MangaId("b".into()))
            .unwrap()
            .unwrap()
            .title,
        "Beta"
    );
    assert!(get_manga(&lib, "mangadex", &MangaId("zzz".into()))
        .unwrap()
        .is_none());
}

#[test]
fn open_creates_file_and_migrates_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib").join("mangas.db");
    open(&path).unwrap();
    let lib = open(&path).unwrap();
    assert!(list_manga(&lib).unwrap().is_empty());
}

#[test]
fn chapters_roundtrip_sorted_by_number() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m", "Manga")).unwrap();
    let chapters = vec![
        chapter("m", "c10", "10"),
        chapter("m", "c2", "2"),
        chapter("m", "c9b", "9.5"),
        chapter("m", "oneshot", "Oneshot"),
    ];
    upsert_chapters(&lib, "mangadex", &chapters).unwrap();
    let listed = list_chapters(&lib, "mangadex", &MangaId("m".into()), None).unwrap();
    let numbers: Vec<&str> = listed.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["2", "9.5", "10", "Oneshot"]);
    assert_eq!(listed[0].title.as_deref(), Some("Chapter 2"));
}

#[test]
fn upsert_refreshes_metadata_but_keeps_local_path() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m", "Manga")).unwrap();
    let id = MangaId("m".into());
    upsert_chapters(&lib, "mangadex", &[chapter("m", "c1", "1")]).unwrap();
    mark_downloaded(
        &lib,
        "mangadex",
        &ChapterId("c1".into()),
        Path::new("/tmp/m/1"),
    )
    .unwrap();

    let mut renamed = chapter("m", "c1", "1");
    renamed.title = Some("New title".into());
    upsert_chapters(&lib, "mangadex", &[renamed]).unwrap();

    let listed = list_chapters(&lib, "mangadex", &id, None).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title.as_deref(), Some("New title"));
    assert_eq!(listed[0].local_path.as_deref(), Some("/tmp/m/1"));
    assert!(listed[0].is_downloaded());
}

#[test]
fn chapters_require_a_followed_manga() {
    let lib = open_in_memory().unwrap();
    assert!(upsert_chapters(&lib, "mangadex", &[chapter("ghost", "c1", "1")]).is_err());
}

#[test]
fn remove_manga_cascades_to_chapters() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m", "Manga")).unwrap();
    upsert_chapters(
        &lib,
        "mangadex",
        &[chapter("m", "c1", "1"), chapter("m", "c2", "2")],
    )
    .unwrap();
    let id = MangaId("m".into());
    assert!(remove_manga(&lib, "mangadex", &id).unwrap());
    assert!(!remove_manga(&lib, "mangadex", &id).unwrap());
    assert!(list_manga(&lib).unwrap().is_empty());
    assert!(list_chapters(&lib, "mangadex", &id, None)
        .unwrap()
        .is_empty());
}

#[test]
fn upsert_prunes_chapters_the_source_dropped_unless_downloaded() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m", "Manga")).unwrap();
    let id = MangaId("m".into());
    let all = [
        chapter("m", "c1", "1"),
        chapter("m", "c2", "2"),
        chapter("m", "c3", "3"),
    ];
    upsert_chapters(&lib, "mangadex", &all).unwrap();
    mark_downloaded(
        &lib,
        "mangadex",
        &ChapterId("c2".into()),
        Path::new("/tmp/2"),
    )
    .unwrap();

    upsert_chapters(&lib, "mangadex", &all[..1]).unwrap();
    let ids: Vec<String> = list_chapters(&lib, "mangadex", &id, None)
        .unwrap()
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(ids, ["c1", "c2"]);
}

#[test]
fn list_chapters_filters_by_language() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m", "Manga")).unwrap();
    let id = MangaId("m".into());
    let mut es = chapter("m", "c1-es", "1");
    es.language = "es".into();
    upsert_chapters(&lib, "mangadex", &[chapter("m", "c1", "1")]).unwrap();
    upsert_chapters(&lib, "mangadex", &[es]).unwrap();

    assert_eq!(list_chapters(&lib, "mangadex", &id, None).unwrap().len(), 2);
    let en = list_chapters(&lib, "mangadex", &id, Some("en")).unwrap();
    assert_eq!(en.len(), 1);
    assert_eq!(en[0].id.0, "c1");
}
