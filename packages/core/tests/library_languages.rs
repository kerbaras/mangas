//! Chapter lists across languages: filtering, per-language refresh, downloads always listed.

use std::path::Path;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, MangaStatus};
use mangas_core::library::{
    list_chapters, mark_downloaded, open_in_memory, upsert_chapters, upsert_manga,
};

fn sample(id: &str) -> Manga {
    Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: "Manga".into(),
        description: String::new(),
        cover_url: None,
        status: MangaStatus::Ongoing,
    }
}

fn chapter(id: &str, number: &str, language: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        manga_id: MangaId("m".into()),
        title: None,
        language: language.into(),
        volume: None,
        number: number.into(),
        local_path: None,
        group: None,
        availability: Default::default(),
    }
}

#[test]
fn list_chapters_filters_by_language() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m")).unwrap();
    let id = MangaId("m".into());
    let es = chapter("c1-es", "1", "es");
    upsert_chapters(&lib, "mangadex", &id, "en", &[chapter("c1", "1", "en")]).unwrap();
    upsert_chapters(&lib, "mangadex", &id, "es", &[es]).unwrap();

    assert_eq!(list_chapters(&lib, "mangadex", &id, None).unwrap().len(), 2);
    let en = list_chapters(&lib, "mangadex", &id, Some("en")).unwrap();
    assert_eq!(en.len(), 1);
    assert_eq!(en[0].id.0, "c1");
}

#[test]
fn empty_refresh_clears_non_downloaded_chapters_of_that_language() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m")).unwrap();
    let id = MangaId("m".into());
    let es = chapter("c1-es", "1", "es");
    upsert_chapters(&lib, "mangadex", &id, "en", &[chapter("c1", "1", "en")]).unwrap();
    upsert_chapters(&lib, "mangadex", &id, "es", &[es]).unwrap();

    upsert_chapters(&lib, "mangadex", &id, "en", &[]).unwrap();
    let left = list_chapters(&lib, "mangadex", &id, None).unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].language, "es");
}

#[test]
fn downloaded_chapters_stay_listed_when_their_language_is_not_asked_for() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &sample("m")).unwrap();
    let id = MangaId("m".into());
    let fr = [chapter("c1-fr", "1", "fr"), chapter("c2-fr", "2", "fr")];
    upsert_chapters(&lib, "mangadex", &id, "fr", &fr).unwrap();
    upsert_chapters(&lib, "mangadex", &id, "en", &[chapter("c1", "1", "en")]).unwrap();
    mark_downloaded(
        &lib,
        "mangadex",
        &ChapterId("c1-fr".into()),
        Path::new("/d/1"),
    )
    .unwrap();

    let en = list_chapters(&lib, "mangadex", &id, Some("en")).unwrap();
    let listed: Vec<(&str, &str)> = en
        .iter()
        .map(|c| (c.id.0.as_str(), c.language.as_str()))
        .collect();
    assert_eq!(listed.len(), 2);
    assert!(listed.contains(&("c1", "en")));
    assert!(listed.contains(&("c1-fr", "fr")));
}
