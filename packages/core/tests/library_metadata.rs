use mangas_core::domain::{Manga, MangaId, MangaStatus};
use mangas_core::library::{get_manga, open, open_in_memory, upsert_manga};

fn credited() -> Manga {
    Manga {
        id: MangaId("solo".into()),
        source: "comick".into(),
        title: "Solo Leveling".into(),
        status: MangaStatus::Completed,
        authors: vec!["Chugong".into(), "h-goon".into()],
        artists: vec!["Jang Sung-Rak".into()],
        year: Some(2018),
        tags: vec!["Action".into(), "Fantasy".into()],
        ..Default::default()
    }
}

fn stored(lib: &mangas_core::library::Library) -> Manga {
    get_manga(lib, "comick", &MangaId("solo".into()))
        .unwrap()
        .unwrap()
}

#[test]
fn credits_year_and_tags_round_trip() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &credited()).unwrap();
    assert_eq!(stored(&lib), credited());
}

#[test]
fn a_refresh_replaces_the_metadata() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &credited()).unwrap();
    let bare = Manga {
        authors: Vec::new(),
        year: None,
        tags: vec!["Drama".into()],
        ..credited()
    };
    upsert_manga(&lib, &bare).unwrap();
    assert_eq!(stored(&lib), bare);
}

#[test]
fn a_manga_without_metadata_survives_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mangas.db");
    let plain = Manga {
        authors: Vec::new(),
        artists: Vec::new(),
        year: None,
        tags: Vec::new(),
        ..credited()
    };
    upsert_manga(&open(&path).unwrap(), &plain).unwrap();
    assert_eq!(stored(&open(&path).unwrap()), plain);
}

#[test]
fn manga_json_without_the_new_fields_still_reads() {
    let old = r#"{"id":"x","source":"mangadex","title":"T","description":"",
        "cover_url":null,"status":"ongoing"}"#;
    let manga: Manga = serde_json::from_str(old).unwrap();
    assert!(manga.authors.is_empty() && manga.tags.is_empty() && manga.year.is_none());
}
