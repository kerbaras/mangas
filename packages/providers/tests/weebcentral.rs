//! WeebCentral mapping on recorded pages (`tests/fixtures/weebcentral/`).

use mangas_core::domain::{ChapterId, Error, MangaId, MangaStatus};
use mangas_providers::weebcentral::{
    cover_url, number_chapters, parse_chapters, parse_label, parse_pages, parse_search,
    parse_series, Label,
};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/weebcentral/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn summary(chapters: &[mangas_core::domain::Chapter]) -> Vec<(String, Option<String>)> {
    chapters
        .iter()
        .map(|c| (c.number.clone(), c.title.clone()))
        .collect()
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
}

#[test]
fn unknown_series_redirects_to_a_page_without_a_title() {
    let id = MangaId("01J76XYCPSY3C4BNPBRY8JMCBX".into());
    let err = parse_series(&id, &fixture("not_found.html")).unwrap_err();
    assert!(matches!(err, Error::NotFound(_)), "{err:?}");
}

#[test]
fn chapter_list_keeps_label_numbers_oldest_first() {
    let id = MangaId("01J76XYCPSY3C4BNPBRY8JMCBE".into());
    let chapters = parse_chapters(&fixture("contract/chapters.html"), &id).unwrap();
    let numbers: Vec<_> = chapters.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["0", "1", "199", "200"]);
    let first = &chapters[0];
    assert_eq!(first.id, ChapterId("01J76XYXYZGGRVGEGATYQWFTD8".into()));
    assert_eq!(
        (first.title.as_deref(), first.language.as_str()),
        (None, "en")
    );
    assert!(chapters
        .iter()
        .all(|c| c.manga_id == id && !c.is_downloaded()));
}

#[test]
fn prologues_keep_their_label_and_no_number() {
    let id = MangaId("01J76XY7EF75DJNQCV04HTPDZK".into());
    let chapters = parse_chapters(&fixture("chapters_prologue.html"), &id).unwrap();
    let expected = [
        ("1", None),
        ("2", None),
        ("350.5", None),
        ("", Some("Prologue 16")),
        ("", Some("Prologue 1")),
    ];
    let expected: Vec<_> = expected
        .iter()
        .map(|(n, t)| (n.to_string(), t.map(str::to_string)))
        .collect();
    assert_eq!(summary(&chapters), expected);
}

#[test]
fn seasons_are_numbered_in_reading_order() {
    let id = MangaId("01J76XY7M0W9WWJ55VJYYB2J1S".into());
    let chapters = parse_chapters(&fixture("chapters_seasons.html"), &id).unwrap();
    let labels = [
        "S1 - Chapter 0",
        "S1 - Chapter 1",
        "S2 - Chapter 1",
        "S3 - Chapter 1",
    ];
    let expected: Vec<_> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| ((i + 1).to_string(), Some(l.to_string())))
        .collect();
    assert_eq!(summary(&chapters), expected);
    assert_eq!(
        chapters[0].id,
        ChapterId("01J76XYZ0MX1T53YGHDX0QZ247".into())
    );
}

#[test]
fn repeated_chapter_links_are_listed_once() {
    let id = MangaId("1".into());
    let listed = vec![
        (ChapterId("B".into()), "Chapter 2".to_string()),
        (ChapterId("A".into()), "Chapter 1".to_string()),
        (ChapterId("B".into()), "Chapter 2".to_string()),
    ];
    assert_eq!(number_chapters(listed, &id).len(), 2);
}

#[test]
fn labels_parse_numbers_seasons_and_titles() {
    let numbered = |n: &str, t: Option<&str>| Label::Numbered {
        number: n.into(),
        title: t.map(Into::into),
    };
    assert_eq!(parse_label("Chapter 12.5"), numbered("12.5", None));
    assert_eq!(parse_label(" Episode 3 "), numbered("3", None));
    assert_eq!(
        parse_label("Chapter 7 - The Gate"),
        numbered("7", Some("The Gate"))
    );
    let season = Label::Season {
        season: 2,
        number: "5".into(),
    };
    assert_eq!(parse_label("S2 - Chapter 5"), season);
    for other in [
        "Prologue 4",
        "Oneshot",
        "Chapter",
        "Chapter x",
        "S2 - Extra",
        "",
    ] {
        assert_eq!(parse_label(other), Label::Other, "{other:?}");
    }
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
