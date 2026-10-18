//! Requested languages, picking one translation per chapter, and multi-language sync.

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, MangaStatus};
use mangas_core::library::{
    fetch_chapters, in_languages, list_chapters, open_in_memory, parse_languages, prefer_languages,
    requested_languages, sync_chapters, upsert_manga, Settings,
};
use mangas_core::sources::{PageUrls, Source};

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
    }
}

fn langs(list: &[&str]) -> Vec<String> {
    list.iter().map(|l| l.to_string()).collect()
}

fn ids(chapters: &[Chapter]) -> Vec<&str> {
    chapters.iter().map(|c| c.id.0.as_str()).collect()
}

#[test]
fn parses_comma_or_space_separated_languages_once_each() {
    assert_eq!(
        parse_languages(" ES-LA, en  pt-br,en,"),
        langs(&["es-la", "en", "pt-br"])
    );
    assert!(parse_languages(" , ").is_empty());
}

#[test]
fn flag_wins_over_saved_languages_which_win_over_english() {
    let saved = Settings {
        preferred_languages: langs(&["es", "en"]),
        ..Settings::default()
    };
    assert_eq!(requested_languages(Some("fr"), &saved), langs(&["fr"]));
    assert_eq!(requested_languages(None, &saved), langs(&["es", "en"]));
    assert_eq!(requested_languages(Some(" "), &saved), langs(&["es", "en"]));
    assert_eq!(
        requested_languages(None, &Settings::default()),
        langs(&["en"])
    );
}

#[test]
fn keeps_requested_languages_and_downloads() {
    let mut owned = chapter("d", "3", "fr");
    owned.local_path = Some("/x".into());
    let all = vec![chapter("a", "1", "en"), chapter("b", "1", "fr"), owned];
    assert_eq!(ids(&in_languages(all, &langs(&["en"]))), ["a", "d"]);
}

#[test]
fn prefers_the_first_language_that_has_each_chapter() {
    let all = vec![
        chapter("es1", "1", "es"),
        chapter("en1", "1", "en"),
        chapter("en2", "2", "en"),
        chapter("es2a", "2.0", "es"),
        chapter("es2b", "2", "es"),
        chapter("en-x", "Oneshot", "en"),
        chapter("es-x", "Oneshot", "es"),
    ];
    let picked = prefer_languages(&all, &langs(&["es", "en"]));
    assert_eq!(ids(&picked), ["es1", "es2a", "es2b", "en-x", "es-x"]);
}

/// Has `es` and `en` chapters; any other language is an error.
struct TwoLanguages;

#[async_trait]
impl Source for TwoLanguages {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>, Error> {
        Ok(vec![])
    }
    async fn manga(&self, id: &MangaId) -> Result<Manga, Error> {
        Err(Error::NotFound(id.0.clone()))
    }
    async fn chapters(&self, _: &MangaId, language: &str) -> Result<Vec<Chapter>, Error> {
        match language {
            "es" | "en" => Ok(vec![chapter(&format!("{language}1"), "1", language)]),
            other => Err(Error::Other(format!("no {other}"))),
        }
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls, Error> {
        Ok(PageUrls(vec![]))
    }
}

#[tokio::test]
async fn skips_failing_languages_unless_all_fail() {
    let id = MangaId("m".into());
    let lists = fetch_chapters(&TwoLanguages, &id, &langs(&["fr", "es", "en"]))
        .await
        .unwrap();
    let got: Vec<&str> = lists.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(got, ["es", "en"]);
    let err = fetch_chapters(&TwoLanguages, &id, &langs(&["fr", "de"]))
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), Error::Other("no fr".into()).to_string());
}

#[tokio::test]
async fn sync_stores_every_language() {
    let lib = open_in_memory().unwrap();
    upsert_manga(
        &lib,
        &Manga {
            id: MangaId("m".into()),
            source: "fake".into(),
            title: "M".into(),
            description: String::new(),
            cover_url: None,
            status: MangaStatus::Ongoing,
        },
    )
    .unwrap();
    let id = MangaId("m".into());
    let synced = sync_chapters(&lib, &TwoLanguages, &id, &langs(&["es", "en"]))
        .await
        .unwrap();
    assert_eq!(ids(&synced), ["es1", "en1"]);
    let stored = list_chapters(&lib, "fake", &id, None).unwrap();
    let mut stored_ids = ids(&stored);
    stored_ids.sort();
    assert_eq!(stored_ids, ["en1", "es1"]);
}
