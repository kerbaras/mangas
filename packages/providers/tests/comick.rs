use mangas_core::domain::{ChapterId, MangaId, MangaStatus};
use mangas_providers::comick::{
    chapter_id, chapter_path, parse_chapter_page, parse_comic_page, parse_reader_page,
    parse_search, parse_status, plain_text, script_json,
};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/comick/contract/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn solo() -> MangaId {
    MangaId("00-solo-leveling".into())
}

#[test]
fn search_results_use_the_slug_as_id() {
    let found = parse_search(&fixture("search.json")).unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].id, solo());
    assert_eq!(found[0].source, "comick");
    assert_eq!(found[0].title, "Solo Leveling");
    assert!(found[0]
        .cover_url
        .as_deref()
        .unwrap()
        .starts_with("https://cdn1.comicknew"));
    assert_eq!(found[1].cover_url, None);
}

#[test]
fn comic_page_maps_title_status_cover_and_plain_description() {
    let manga = parse_comic_page(&solo(), &fixture("manga.html")).unwrap();
    assert_eq!(manga.id, solo());
    assert_eq!(manga.title, "Solo Leveling");
    assert_eq!(manga.status, MangaStatus::Completed);
    assert!(manga.description.starts_with("10 years ago"));
    assert!(!manga.description.contains('<'));
    assert!(manga.cover_url.unwrap().ends_with("/covers/48810d64.webp"));
}

#[test]
fn page_without_comic_data_is_not_found() {
    let err = parse_comic_page(&solo(), "<html><body>gone</body></html>").unwrap_err();
    assert!(err.to_string().contains("00-solo-leveling"), "{err}");
}

#[test]
fn chapter_page_filters_language_and_reports_last_page() {
    let (chapters, last) = parse_chapter_page(&fixture("chapters.json"), &solo(), "en").unwrap();
    assert_eq!(last, 1);
    let numbers: Vec<_> = chapters.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["200.5", "193", "0"]);
    let side = &chapters[1];
    assert_eq!(
        side.id,
        ChapterId("67H16VGw-chapter-193-en@00-solo-leveling".into())
    );
    assert_eq!(side.title.as_deref(), Some("Side Story 14"));
    assert_eq!(side.volume.as_deref(), Some("3"));
    assert_eq!(side.group.as_deref(), Some("Flame Scans"));
    assert_eq!(chapters[0].title, None);
    assert_eq!(chapters[0].volume, None);
    let (spanish, _) = parse_chapter_page(&fixture("chapters.json"), &solo(), "es-419").unwrap();
    assert_eq!(spanish.len(), 1);
    assert_eq!(spanish[0].language, "es-419");
}

#[test]
fn chapter_ids_round_trip_to_reader_paths() {
    let id = chapter_id(&solo(), "lkRE7", "0", "en");
    assert_eq!(id.0, "lkRE7-chapter-0-en@00-solo-leveling");
    assert_eq!(
        chapter_path(&id).unwrap(),
        "/comic/00-solo-leveling/lkRE7-chapter-0-en"
    );
    assert!(chapter_path(&ChapterId("lkRE7".into())).is_err());
    assert!(chapter_path(&ChapterId("lkRE7-chapter-0-en@".into())).is_err());
    assert!(chapter_path(&ChapterId("lkRE7-chapter-0-en@a/b".into())).is_err());
}

#[test]
fn same_numbered_releases_differ_in_the_first_eight_chars() {
    let a = chapter_id(&solo(), "5N_wGCXG", "200", "en");
    let b = chapter_id(&solo(), "h94WANth", "200", "en");
    assert_ne!(a.0[..8], b.0[..8]);
}

#[test]
fn reader_page_lists_absolute_image_urls() {
    let id = ChapterId("lkRE7-chapter-0-en@00-solo-leveling".into());
    let pages = parse_reader_page(&id, &fixture("chapter.html")).unwrap();
    assert_eq!(pages.0.len(), 2);
    assert!(pages.0[0].starts_with("https://cdn1.comicknew.pictures/00-solo-leveling/"));
    assert!(parse_reader_page(&id, "<html></html>").is_err());
}

#[test]
fn status_codes_map_to_manga_status() {
    assert_eq!(parse_status(Some(1)), MangaStatus::Ongoing);
    assert_eq!(parse_status(Some(3)), MangaStatus::Cancelled);
    assert_eq!(parse_status(Some(4)), MangaStatus::Hiatus);
    assert_eq!(parse_status(None), MangaStatus::Unknown);
}

#[test]
fn script_json_finds_the_tag_by_id() {
    let html =
        r#"<script id="a">1</script><script type="application/json" id="b"> {"x":2} </script>"#;
    assert_eq!(script_json(html, "b"), Some(r#"{"x":2}"#));
    assert_eq!(script_json(html, "c"), None);
}

#[test]
fn plain_text_drops_tags_and_decodes_entities() {
    let html = "<p class=\"x\">Tom &amp; Jerry<br>  run</p><p>&quot;fast&quot;</p>";
    assert_eq!(plain_text(html), "Tom & Jerry\nrun\n\"fast\"");
}
