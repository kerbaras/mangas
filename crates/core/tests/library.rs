use mangas_core::domain::{Manga, MangaId, MangaStatus};
use mangas_core::library::{list_manga, open, open_in_memory, upsert_manga};

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

#[test]
fn upsert_then_list_roundtrips() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("a", "Alpha")).unwrap();
    upsert_manga(&lib, &sample("a", "Alpha (renamed)")).unwrap();
    upsert_manga(&lib, &sample("b", "Beta")).unwrap();
    let all = list_manga(&lib).unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|m| m.title == "Alpha (renamed)"));
}

#[test]
fn open_creates_file_and_migrates_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib").join("mangas.db");
    open(&path).unwrap();
    let lib = open(&path).unwrap();
    assert!(list_manga(&lib).unwrap().is_empty());
}
