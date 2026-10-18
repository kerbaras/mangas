use mangas_core::domain::Chapter;
use mangas_core::domain::{Availability, Error, MangaId, MangaStatus};
use mangas_providers::mangadex::{
    cover_url, dedupe_chapters, parse_at_home_response, parse_error_response, parse_feed_response,
    parse_manga_response, parse_search_response, FeedChapter,
};

const SOLO_LEVELING: &str = "32d76d19-8a05-4db0-9fc2-e0b0648fe9d0";
const EMINENCE: &str = "91d928a1-7237-4d03-abc3-4b1440cc7d55";

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/mangadex/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn dedupe_and_sort(entries: Vec<FeedChapter>) -> Vec<Chapter> {
    dedupe_chapters(entries, &[])
}

#[test]
fn search_maps_titles_status_and_covers() {
    let results = parse_search_response(&fixture("search.json")).unwrap();
    assert_eq!(results.len(), 3);
    let solo = results.iter().find(|m| m.id.0 == SOLO_LEVELING).unwrap();
    assert_eq!(solo.source, "mangadex");
    assert_eq!(solo.title, "Solo Leveling");
    assert_eq!(solo.status, MangaStatus::Completed);
    assert!(
        solo.description.starts_with("10 years ago"),
        "{}",
        solo.description
    );
    let cover = solo.cover_url.as_deref().unwrap();
    assert!(cover.starts_with(&format!(
        "https://uploads.mangadex.org/covers/{SOLO_LEVELING}/"
    )));
    assert!(cover.ends_with(".jpg.256.jpg"), "256px thumbnail: {cover}");
    assert!(results.iter().all(|m| m.cover_url.is_some()));
}

#[test]
fn manga_by_id_builds_cover_from_relationship() {
    let manga = parse_manga_response(&fixture("manga.json")).unwrap();
    assert_eq!(manga.id, MangaId(SOLO_LEVELING.into()));
    assert_eq!(manga.title, "Solo Leveling");
    assert_eq!(
        manga.cover_url.as_deref(),
        Some(cover_url(SOLO_LEVELING, "e90bdc47-c8b9-4df7-b2c0-17641b645ee1.jpg").as_str())
    );
    assert_eq!(manga.status, MangaStatus::Completed);
}

#[test]
fn feed_lists_externally_hosted_chapters_as_external() {
    let id = MangaId(SOLO_LEVELING.into());
    let page = parse_feed_response(&fixture("feed.json"), &id, "en").unwrap();
    assert_eq!(page.chapters.len(), 5, "every upload has an externalUrl");
    let first = &page.chapters[0].chapter;
    assert_eq!(first.number, "0");
    assert_eq!(
        first.availability,
        Availability::External {
            url: Some("https://www.webnovel.com/comic/15227640605485101/45196186038101142".into())
        }
    );
    assert!(page.chapters.iter().all(|c| !c.chapter.is_available()));
    assert_eq!(page.next_offset, Some(5), "limit 5 of total 24");
}

#[test]
fn feed_maps_chapters_and_dedupes_by_number() {
    let id = MangaId(EMINENCE.into());
    let page = parse_feed_response(&fixture("feed_hosted.json"), &id, "en").unwrap();
    assert_eq!(
        page.chapters.len(),
        6,
        "three uploads of ch1, two of 2.1, one of 2.2"
    );
    let first = &page.chapters[0].chapter;
    assert_eq!(first.id.0, "6573cb1d-8153-4c2b-aadf-ccde663c50c6");
    assert_eq!(first.manga_id, id);
    assert_eq!(first.number, "1");
    assert_eq!(first.volume.as_deref(), Some("1"));
    assert_eq!(first.language, "en");
    assert_eq!(first.title, None, "empty titles become None");
    assert!(!first.is_downloaded());

    let unique = dedupe_and_sort(page.chapters);
    let numbers: Vec<&str> = unique.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["1", "2.1", "2.2"]);
    assert_eq!(
        unique[0].id.0, "6573cb1d-8153-4c2b-aadf-ccde663c50c6",
        "most recent upload wins"
    );
}

#[test]
fn feed_filters_other_languages() {
    let id = MangaId(EMINENCE.into());
    let page = parse_feed_response(&fixture("feed_hosted.json"), &id, "es-la").unwrap();
    assert!(page.chapters.is_empty());
}

#[test]
fn dedupe_sorts_numerically_not_lexically() {
    let id = MangaId(EMINENCE.into());
    let mut chapters = parse_feed_response(&fixture("feed_hosted.json"), &id, "en")
        .unwrap()
        .chapters;
    chapters[1].chapter.number = "10".into();
    chapters[2].chapter.number = "9.5".into();
    let numbers: Vec<String> = dedupe_and_sort(chapters)
        .into_iter()
        .map(|c| c.number)
        .collect();
    assert_eq!(numbers, ["1", "2.1", "2.2", "9.5", "10"]);
}

#[test]
fn dedupe_keeps_every_unnumbered_chapter() {
    let id = MangaId(EMINENCE.into());
    let mut chapters = parse_feed_response(&fixture("feed_hosted.json"), &id, "en")
        .unwrap()
        .chapters;
    chapters[1].chapter.number = String::new();
    chapters[2].chapter.number = String::new();
    let unique = dedupe_and_sort(chapters);
    assert_eq!(unique.len(), 5);
    assert_eq!(unique.iter().filter(|c| c.number.is_empty()).count(), 2);
}

#[test]
fn at_home_builds_full_quality_page_urls() {
    let pages = parse_at_home_response(&fixture("at_home.json"), false).unwrap();
    assert_eq!(pages.0.len(), 30);
    assert_eq!(
        pages.0[0],
        "https://cmdxd98sb0x3yprd.mangadex.network/data/b0d37a668e9a108f695f80fa7260916d/1-983c0c4bace61385bcde1beb39f76b05071ea40ca0b43d47e13d810a18acf5f5.png"
    );
    assert!(pages
        .0
        .iter()
        .all(|u| u.contains("/data/") && !u.contains("/data-saver/")));
}

#[test]
fn api_errors_become_source_errors_with_detail() {
    let err = parse_error_response(404, &fixture("error_404.json"));
    match err {
        Error::Source { name, message } => {
            assert_eq!(name, "mangadex");
            assert!(message.contains("not found"), "{message}");
        }
        other => panic!("expected Error::Source, got {other:?}"),
    }
    assert_eq!(
        parse_error_response(503, "<html>").to_string(),
        "source mangadex: HTTP 503"
    );
}

#[test]
fn malformed_json_is_a_source_error() {
    assert!(matches!(
        parse_search_response("{"),
        Err(Error::Source { .. })
    ));
}
