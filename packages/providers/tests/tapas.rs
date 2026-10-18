use mangas_core::domain::{Availability, Chapter, ChapterId, MangaId, MangaStatus};
use mangas_providers::tapas::{
    next_episode_page, parse_episodes, parse_pages, parse_search, parse_series, to_chapters,
    MAX_EPISODE_PAGES,
};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tapas");

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{FIXTURES}/{name}")).unwrap()
}

fn series() -> MangaId {
    MangaId("2101".into())
}

#[test]
fn search_results_map_to_manga() {
    let found = parse_search(&fixture("contract/search.html"));
    assert_eq!(found.len(), 1);
    let m = &found[0];
    assert_eq!(m.id, series());
    assert_eq!(m.source, "tapas");
    assert_eq!(m.title, "Politidoodles");
    assert_eq!(m.description, "News-themed cartoons twice a week.");
    assert!(m
        .cover_url
        .as_deref()
        .unwrap()
        .starts_with("https://us-a.tapas.io/"));
}

#[test]
fn search_without_results_is_empty() {
    assert!(parse_search("<html><body><p>No results</p></body></html>").is_empty());
}

#[test]
fn series_page_maps_to_manga() {
    let m = parse_series(&series(), &fixture("contract/series.html")).unwrap();
    assert_eq!(m.title, "Politidoodles");
    assert_eq!(m.description, "News-themed cartoons twice a week.");
    assert_eq!(m.status, MangaStatus::Unknown);
    assert!(m.cover_url.unwrap().ends_with(".jpg"));
    assert_eq!(m.authors, ["babylonsticks"]);
    assert_eq!(m.tags, ["Slice of life"]);
    assert!(m.artists.is_empty() && m.year.is_none());
}

#[test]
fn series_title_falls_back_to_og_site_name() {
    let html = r#"<meta property="og:site_name" content="Read Tom &amp; Jerry"/>"#;
    assert_eq!(parse_series(&series(), html).unwrap().title, "Tom & Jerry");
    assert!(parse_series(&series(), "<html></html>").is_err());
}

#[test]
fn episodes_report_pagination() {
    let (first, more) = parse_episodes(&fixture("contract/episodes_1.json")).unwrap();
    assert_eq!(first.len(), 2);
    assert!(more);
    let (last, more) = parse_episodes(&fixture("contract/episodes_2.json")).unwrap();
    assert_eq!(last.len(), 2);
    assert!(!more);
    assert!(parse_episodes("<html>sign in</html>").is_err());
}

#[test]
fn free_episodes_become_numbered_english_chapters() {
    let (episodes, _) = parse_episodes(&fixture("contract/episodes_1.json")).unwrap();
    let chapters = to_chapters(&episodes, &series());
    assert!(chapters.iter().all(|c| c.is_available()));
    assert_eq!(chapters[0].id, ChapterId("21321".into()));
    assert_eq!(chapters[0].number, "1");
    assert_eq!(chapters[0].language, "en");
    assert_eq!(chapters[0].manga_id, series());
    assert!(chapters[0].title.is_some());
}

fn availability_by_number(chapters: &[Chapter]) -> Vec<(&str, &Availability)> {
    chapters
        .iter()
        .map(|c| (c.number.as_str(), &c.availability))
        .collect()
}

#[test]
fn paid_episodes_are_listed_as_locked_in_site_order() {
    let (episodes, _) = parse_episodes(&fixture("episodes_locked.json")).unwrap();
    let chapters = to_chapters(&episodes, &MangaId("205841".into()));
    assert_eq!(chapters.len(), 3, "no episode is dropped");
    let paid = chapters
        .iter()
        .find(|c| c.id == ChapterId("2140071".into()));
    assert_eq!(paid.unwrap().availability, Availability::Locked);
    assert_eq!(paid.unwrap().number, "3", "keeps the site's numbering");
    assert_eq!(chapters.iter().filter(|c| c.is_available()).count(), 2);
}

#[test]
fn early_access_episodes_are_listed_as_locked() {
    let (episodes, _) = parse_episodes(&fixture("episodes_early_access.json")).unwrap();
    let chapters = to_chapters(&episodes, &MangaId("171693".into()));
    let early = chapters
        .iter()
        .find(|c| c.id == ChapterId("4009950".into()));
    assert_eq!(early.unwrap().availability, Availability::Locked);
    assert_eq!(early.unwrap().number, "686", "numbered by pending_scene");
    assert_eq!(chapters.iter().filter(|c| c.is_available()).count(), 1);
}

#[test]
fn contract_list_keeps_the_paid_episode_after_the_free_ones() {
    let (mut episodes, _) = parse_episodes(&fixture("contract/episodes_1.json")).unwrap();
    episodes.extend(
        parse_episodes(&fixture("contract/episodes_2.json"))
            .unwrap()
            .0,
    );
    let chapters = to_chapters(&episodes, &series());
    assert_eq!(
        availability_by_number(&chapters),
        [
            ("1", &Availability::Available),
            ("2", &Availability::Available),
            ("21", &Availability::Available),
            ("22", &Availability::Locked),
        ]
    );
}

#[test]
fn episode_viewer_maps_to_page_urls() {
    let pages = parse_pages(
        &ChapterId("21321".into()),
        &fixture("contract/episode.html"),
    )
    .unwrap();
    assert_eq!(pages.0.len(), 1);
    assert!(pages.0[0].starts_with("https://us-a.tapas.io/"));
    assert!(
        pages.0[0].contains("__token__=exp="),
        "signed URL kept intact"
    );
}

#[test]
fn locked_episode_viewer_is_an_error() {
    let err = parse_pages(
        &ChapterId("4009950".into()),
        &fixture("episode_locked.html"),
    );
    assert!(err.unwrap_err().to_string().contains("locked"));
}

#[test]
fn episode_pages_continue_while_has_next() {
    assert_eq!(
        next_episode_page(&series(), 1, true, 20, 500).unwrap(),
        Some(2)
    );
    assert_eq!(
        next_episode_page(&series(), 7, false, 3, 500).unwrap(),
        None
    );
    assert_eq!(next_episode_page(&series(), 7, true, 0, 500).unwrap(), None);
}

#[test]
fn episode_list_past_the_cap_is_an_error_not_a_truncated_list() {
    let cap = MAX_EPISODE_PAGES;
    assert_eq!(
        next_episode_page(&series(), cap - 1, true, 20, cap).unwrap(),
        Some(cap)
    );
    let err = next_episode_page(&series(), cap, true, 20, cap).unwrap_err();
    assert!(
        err.to_string().contains("more than 500 episode pages"),
        "{err}"
    );
    assert_eq!(
        next_episode_page(&series(), cap, false, 20, cap).unwrap(),
        None
    );
}
