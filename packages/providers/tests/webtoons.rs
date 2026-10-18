use mangas_core::domain::{ChapterId, MangaId, MangaStatus};
use mangas_providers::webtoons::{
    cover_url, full_size, parse_canvas_search, parse_episode_page, parse_search, parse_title_page,
    parse_viewer, supported, to_chapters, unescape, Episode, Title,
};

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/webtoons/contract"
);

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{FIXTURES}/{name}")).unwrap()
}

#[test]
fn title_ids_round_trip_and_bare_numbers_are_english_originals() {
    for (id, title) in [
        ("en-webtoon-95", Title::original("en", 95)),
        ("zh-hant-canvas-123", Title::canvas("zh-hant", 123)),
        ("95", Title::original("en", 95)),
    ] {
        assert_eq!(Title::parse(&MangaId(id.into())).unwrap(), title, "{id}");
    }
    assert_eq!(
        Title::canvas("zh-hant", 1).id(),
        MangaId("zh-hant-canvas-1".into())
    );
    for bad in ["", "en-manga-1", "en-webtoon-x", "-webtoon-1", "webtoon-1"] {
        assert!(Title::parse(&MangaId(bad.into())).is_err(), "{bad:?}");
    }
}

#[test]
fn episode_ids_round_trip_and_name_the_viewer() {
    let id = ChapterId("zh-hant-canvas-726081-12".into());
    let episode = Episode::parse(&id).unwrap();
    assert_eq!(episode.title, Title::canvas("zh-hant", 726081));
    assert_eq!(episode.number, 12);
    assert_eq!(episode.id(), id);
    assert_eq!(episode.viewer_path(), "/zh-hant/canvas/a/a/viewer");
    let original = Episode::parse(&ChapterId("en-webtoon-95-1".into())).unwrap();
    assert_eq!(original.viewer_path(), "/en/a/a/a/viewer");
    assert!(Episode::parse(&ChapterId("en-webtoon-95".into())).is_err());
}

#[test]
fn episode_queries_page_and_name_the_canvas_language() {
    let canvas = Title::canvas("es", 7);
    assert_eq!(canvas.episodes_path(), "/api/v1/canvas/7/episodes");
    assert_eq!(canvas.list_path(), "/challenge/episodeList");
    let query = canvas.episodes_query(1000, Some(5));
    assert!(query.contains(&("cursor", "5".into())));
    assert!(query.contains(&("readingLanguageCode", "es".into())));
    let original = Title::original("en", 95).episodes_query(1000, None);
    assert_eq!(original, vec![("pageSize", "1000".to_string())]);
}

#[test]
fn search_maps_originals_with_referer_free_covers() {
    let found = parse_search(&fixture("search.json"), "en").unwrap();
    assert_eq!(found[0].id, MangaId("en-webtoon-8136".into()));
    assert_eq!(found[0].title, "Tower of God: Urek Mazino");
    assert_eq!(found[0].description, "By SIU");
    let cover = found[0].cover_url.as_deref().unwrap();
    assert!(
        cover.starts_with("https://swebtoon-phinf.pstatic.net/2025"),
        "{cover}"
    );
    assert!(parse_search(r#"{"result":null}"#, "en").unwrap().is_empty());
    let mixed = r#"{"result":{"searchedList":[
        {"titleNo":null,"title":null,"searchMode":"AUTHOR","authorName":"SIU"},
        {"titleNo":8591,"title":"神之塔外傳：烏雷克&middot;馬奇努","searchMode":"TITLE"}]}}"#;
    let found = parse_search(mixed, "zh-hant").unwrap();
    assert_eq!(found.len(), 1, "author entries are skipped");
    assert_eq!(found[0].title, "神之塔外傳：烏雷克·馬奇努");
    assert!(parse_search("<html>", "en").is_err());
}

#[test]
fn canvas_search_reads_challenge_cards_only() {
    let found = parse_canvas_search(&fixture("canvas.html"), "en");
    let ids: Vec<_> = found.iter().map(|m| m.id.0.as_str()).collect();
    assert_eq!(ids, ["en-canvas-726081", "en-canvas-908587"]);
    assert_eq!(found[0].title, "Tower Of God (No Mans Tower)");
    let cover = found[0].cover_url.as_deref().unwrap();
    assert!(
        cover.starts_with("https://swebtoon-phinf.pstatic.net/"),
        "{cover}"
    );
    let original =
        r#"<a data-title-no="1" data-webtoon-type="WEBTOON"><strong class="title">X</strong></a>"#;
    assert!(parse_canvas_search(original, "en").is_empty());
}

#[test]
fn title_page_gives_metadata_and_status() {
    let id = MangaId("en-webtoon-8136".into());
    let manga = parse_title_page(&id, &fixture("title.html")).unwrap();
    assert_eq!(manga.id, id);
    assert_eq!(manga.title, "Tower of God: Urek Mazino");
    assert!(manga.description.contains("Ranker, Urek Mazino—but once"));
    assert_eq!(manga.status, MangaStatus::Ongoing);
    assert!(manga
        .cover_url
        .unwrap()
        .starts_with("https://swebtoon-phinf.pstatic.net/"));
    let done =
        r#"<meta property="og:title" content="A &amp; B" /><span class="txt_ico_completed">"#;
    let manga = parse_title_page(&id, done).unwrap();
    assert_eq!(
        (manga.title.as_str(), manga.status),
        ("A & B", MangaStatus::Completed)
    );
    assert!(parse_title_page(&id, "<html></html>").is_err());
}

#[test]
fn episodes_become_sorted_chapters_in_the_title_language() {
    let title = Title::canvas("fr", 3);
    let id = title.id();
    let body = r#"{"result":{"episodeList":[
        {"episodeNo":10,"episodeTitle":"Ep. 10"},{"episodeNo":2,"episodeTitle":" "},
        {"episodeNo":10,"episodeTitle":"Ep. 10"}],"nextCursor":0}}"#;
    let page = parse_episode_page(body, &title).unwrap();
    let chapters = to_chapters(&id, &title, vec![page]);
    let numbers: Vec<_> = chapters.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["2", "10"]);
    assert_eq!(chapters[0].id, ChapterId("fr-canvas-3-2".into()));
    assert_eq!(chapters[0].title, None);
    assert_eq!(chapters[1].title.as_deref(), Some("Ep. 10"));
    assert!(chapters
        .iter()
        .all(|c| c.language == "fr" && c.manga_id == id));
    let missing = parse_episode_page(r#"{"result":null}"#, &title);
    assert!(matches!(
        missing,
        Err(mangas_core::domain::Error::NotFound(_))
    ));
}

#[test]
fn viewer_lists_full_size_page_images_only() {
    let pages = parse_viewer(&fixture("viewer.html")).unwrap().0;
    assert_eq!(pages.len(), 3);
    assert!(pages
        .iter()
        .all(|p| p.starts_with("https://webtoon-phinf.pstatic.net/")));
    assert!(pages.iter().all(|p| !p.contains("type=")), "{pages:?}");
    assert!(pages[0].ends_with("_0001.jpg"));
    let motion = r#"<div id="ozViewer"></div>"#;
    let err = parse_viewer(motion).unwrap_err().to_string();
    assert!(err.contains("motion"), "{err}");
    assert!(parse_viewer("<html></html>").is_err());
}

#[test]
fn helpers_normalise_urls_text_and_languages() {
    assert_eq!(
        full_size("https://x.net/a.jpg?type=q90"),
        "https://x.net/a.jpg"
    );
    assert_eq!(
        full_size("https://x.net/a.jpg?type=q90&v=2"),
        "https://x.net/a.jpg?v=2"
    );
    assert_eq!(
        cover_url("/a.png"),
        "https://swebtoon-phinf.pstatic.net/a.png"
    );
    assert_eq!(
        cover_url("https://other.net/a.png"),
        "https://other.net/a.png"
    );
    assert_eq!(
        unescape(" Tom &amp; Jerry&#39;s &#x2014; &bogus; & "),
        "Tom & Jerry's — &bogus; &"
    );
    let preferred = ["ja", "zh-hant", "en"].map(String::from);
    assert_eq!(supported(&preferred), ["zh-hant", "en"]);
}
