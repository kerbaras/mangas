use mangas_core::domain::{ChapterId, MangaId, MangaStatus};
use mangas_providers::tapas::{
    parse_episodes, parse_pages, parse_search, parse_series, to_chapters,
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
    assert_eq!(last.len(), 1);
    assert!(!more);
    assert!(parse_episodes("<html>sign in</html>").is_err());
}

#[test]
fn free_episodes_become_numbered_english_chapters() {
    let (episodes, _) = parse_episodes(&fixture("contract/episodes_1.json")).unwrap();
    let (chapters, locked) = to_chapters(&episodes, &series());
    assert_eq!(locked, 0);
    assert_eq!(chapters[0].id, ChapterId("21321".into()));
    assert_eq!(chapters[0].number, "1");
    assert_eq!(chapters[0].language, "en");
    assert_eq!(chapters[0].manga_id, series());
    assert!(chapters[0].title.is_some());
}

#[test]
fn paid_episodes_are_skipped_and_counted() {
    let (episodes, _) = parse_episodes(&fixture("episodes_locked.json")).unwrap();
    let (chapters, locked) = to_chapters(&episodes, &MangaId("205841".into()));
    assert_eq!((chapters.len(), locked), (2, 1));
    assert!(chapters.iter().all(|c| c.id != ChapterId("2140071".into())));
}

#[test]
fn early_access_episodes_are_skipped_and_counted() {
    let (episodes, _) = parse_episodes(&fixture("episodes_early_access.json")).unwrap();
    let (chapters, locked) = to_chapters(&episodes, &MangaId("171693".into()));
    assert_eq!((chapters.len(), locked), (1, 1));
    assert!(chapters.iter().all(|c| c.id != ChapterId("4009950".into())));
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
