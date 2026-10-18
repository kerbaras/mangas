//! WeebCentral mapping on recorded pages (`tests/fixtures/weebcentral/`); chapter lists are in
//! `weebcentral_chapters.rs`.

use mangas_core::domain::{Error, MangaId, MangaStatus};
use mangas_providers::weebcentral::{cover_url, parse_pages, parse_search, parse_series};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/weebcentral/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn search_maps_minimal_display_articles() {
    let found = parse_search(&fixture("contract/search.html")).unwrap();
    let titles: Vec<_> = found.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Solo Leveling",
            "Solo Leveling (Volume)",
            "Solo Leveling: Ragnarok"
        ]
    );
    let first = &found[0];
    assert_eq!(first.id, MangaId("01J76XYCPSY3C4BNPBRY8JMCBE".into()));
    assert_eq!(first.source, "weebcentral");
    assert_eq!(first.status, MangaStatus::Completed);
    assert_eq!(found[2].status, MangaStatus::Hiatus);
    assert_eq!(
        first.cover_url.as_deref(),
        Some(cover_url(&first.id.0).as_str())
    );
}

#[test]
fn search_without_results_is_empty() {
    assert!(parse_search("\n\n").unwrap().is_empty());
}

#[test]
fn series_page_maps_title_status_description_and_cover() {
    let id = MangaId("01J76XYCPSY3C4BNPBRY8JMCBE".into());
    let manga = parse_series(&id, &fixture("contract/series.html")).unwrap();
    assert_eq!(manga.title, "Solo Leveling");
    assert_eq!(manga.status, MangaStatus::Completed);
    assert!(manga.description.starts_with("E-class hunter Jinwoo Sung"));
    // The series page and the search derive the same cover.
    assert_eq!(manga.cover_url, Some(cover_url(&id.0)));
    assert_eq!(
        manga.authors,
        ["GEE So-Lyung", "JANG Sung-Lak", "Redice Studio"]
    );
    assert_eq!(manga.tags, ["Action", "Adventure", "Fantasy", "Shounen"]);
    assert_eq!(manga.year, Some(2018));
}

#[test]
fn unknown_series_redirects_to_a_page_without_a_title() {
    let id = MangaId("01J76XYCPSY3C4BNPBRY8JMCBX".into());
    let err = parse_series(&id, &fixture("not_found.html")).unwrap_err();
    assert!(matches!(err, Error::NotFound(_)), "{err:?}");
}

#[test]
fn pages_are_the_long_strip_images_in_order() {
    let pages = parse_pages(&fixture("contract/images.html")).unwrap().0;
    assert_eq!(pages.len(), 12);
    assert!(
        pages[0].starts_with("https://") && pages[0].ends_with("001.png"),
        "{}",
        pages[0]
    );
    assert!(pages[11].ends_with("012.png"), "{}", pages[11]);
}

#[test]
fn relative_page_sources_resolve_against_the_site() {
    let body = r#"<img src="/manga/x/1.png" alt="Page 1"><img src="/x.png" alt="Page 2" x-show="false"><img src="/logo.png" alt="logo">"#;
    let pages = parse_pages(body).unwrap().0;
    assert_eq!(pages, ["https://weebcentral.com/manga/x/1.png"]);
}
