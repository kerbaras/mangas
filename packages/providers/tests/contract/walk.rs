//! The data contract: search → manga → chapters → pages, asserted at each step.

use std::cmp::Ordering;
use std::collections::HashSet;

use mangas_core::domain::{compare_numbers, parse_number, Availability, Chapter, Manga};
use mangas_providers::{ProviderInfo, Source};
use url::Url;

/// What a walk saw, for further checks (page headers).
pub struct Walked {
    pub manga: Manga,
    pub chapters: Vec<Chapter>,
    pub pages: Vec<String>,
}

/// Search results tried until one has chapters in the language.
const CANDIDATES: usize = 5;

pub async fn assert_walk(
    source: &dyn Source,
    info: &ProviderInfo,
    query: &str,
    language: &str,
) -> Walked {
    let id = info.id;
    assert_eq!(source.name(), id, "Source::name must be the provider id");
    let found = source.search(query, 10).await;
    let found = found.unwrap_or_else(|e| panic!("{id}: search: {e}"));
    assert_search(id, &found);

    let manga = source.manga(&found[0].id).await;
    let manga = manga.unwrap_or_else(|e| panic!("{id}: manga: {e}"));
    assert_eq!(
        manga.id, found[0].id,
        "{id}: manga(id) must round-trip the id"
    );
    assert_eq!(manga.source, id, "{id}: manga source");
    assert!(
        !manga.title.trim().is_empty(),
        "{id}: manga without a title"
    );

    let (owner, chapters) = first_with_chapters(source, id, &found, language).await;
    assert_chapters(id, &owner, language, &chapters);

    let first = chapters.iter().find(|c| c.is_available());
    let first = first.unwrap_or_else(|| panic!("{id}: no downloadable chapter"));
    let pages = source.pages(&first.id).await;
    let pages = pages.unwrap_or_else(|e| panic!("{id}: pages: {e}")).0;
    assert_pages(id, &pages);
    Walked {
        manga,
        chapters,
        pages,
    }
}

fn assert_search(id: &str, found: &[Manga]) {
    assert!(!found.is_empty(), "{id}: search found nothing");
    let ids: HashSet<_> = found.iter().map(|m| &m.id).collect();
    assert_eq!(
        ids.len(),
        found.len(),
        "{id}: duplicate ids in search results"
    );
    for m in found {
        assert!(!m.id.0.is_empty(), "{id}: search result without an id");
        assert!(!m.title.trim().is_empty(), "{id}: {} has no title", m.id);
        assert_eq!(m.source, id, "{id}: {} has source {}", m.id, m.source);
    }
}

async fn first_with_chapters(
    source: &dyn Source,
    id: &str,
    found: &[Manga],
    language: &str,
) -> (Manga, Vec<Chapter>) {
    for manga in found.iter().take(CANDIDATES) {
        let chapters = source.chapters(&manga.id, language).await;
        let chapters = chapters.unwrap_or_else(|e| panic!("{id}: chapters: {e}"));
        if !chapters.is_empty() {
            return (manga.clone(), chapters);
        }
    }
    panic!("{id}: no {language} chapters in the first {CANDIDATES} results");
}

fn assert_chapters(id: &str, owner: &Manga, language: &str, chapters: &[Chapter]) {
    let ids: HashSet<_> = chapters.iter().map(|c| &c.id).collect();
    assert_eq!(ids.len(), chapters.len(), "{id}: duplicate chapter ids");
    for c in chapters {
        assert_eq!(
            c.manga_id, owner.id,
            "{id}: chapter {} of another manga",
            c.id
        );
        assert_eq!(
            c.language, language,
            "{id}: chapter {} ignores the language",
            c.id
        );
        assert!(
            c.number.is_empty() || parse_number(&c.number).is_some(),
            "{id}: chapter {} number {:?} does not parse",
            c.id,
            c.number
        );
        assert!(
            !c.is_downloaded(),
            "{id}: fresh chapter {} marked downloaded",
            c.id
        );
        if let Availability::External { url: Some(url) } = &c.availability {
            assert!(
                Url::parse(url).is_ok_and(|u| u.scheme() == "https"),
                "{id}: chapter {} external url {url:?}",
                c.id
            );
        }
    }
    let sorted = chapters
        .windows(2)
        .all(|w| compare_numbers(&w[0].number, &w[1].number) != Ordering::Greater);
    let numbers: Vec<_> = chapters.iter().map(|c| c.number.as_str()).collect();
    assert!(sorted, "{id}: chapters not sorted by number: {numbers:?}");
}

fn assert_pages(id: &str, pages: &[String]) {
    assert!(!pages.is_empty(), "{id}: chapter without pages");
    for page in pages {
        let url = Url::parse(page).unwrap_or_else(|e| panic!("{id}: page {page:?}: {e}"));
        assert!(
            matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
            "{id}: page {page} is not an absolute http(s) URL"
        );
    }
}
