use mangas_core::domain::{ChapterId, MangaId, MangaStatus};
use mangas_providers::mangadot::{
    images_path, loader_status, matches, parse_chapters, parse_images, parse_manga_list,
    parse_manga_page, site_url,
};

const SEARCH: &str = r#"{"manga_list":[{"id":32807,"title":"Konyaku Hakisarete","genres":["Comedy"],"status":"Ongoing","photo":"/uploads/ea2cc63c.webp","date_added":"2026-06-30 07:25:30+00","description":"The protagonist"}]}"#;

const CHAPTERS: &str = r#"[
 {"id":616677,"chapter_number":1,"volume_number":null,"chapter_title":"Chapter 1","language":"en","group_id":0,"group_name":null,"date_added":"2026-03-05 06:48:52+00","page_count":37,"source":"scraper","scanlator_name":null,"groups":[]},
 {"id":930334,"chapter_number":"2.50","volume_number":1,"chapter_title":"","language":"en","source":"user"},
 {"id":616699,"chapter_number":3,"language":"es","source":"scraper"}
]"#;

const IMAGES: &str = r#"{"chapter":{"id":762327,"manga_id":18394,"chapter_number":"2.0","page_count":2},"manga":{"id":18394,"title":"From Eroica with Love"},"images":[{"url":"/chapters/manga_18394/chapter_2/001.webp","w":1200,"h":906},{"url":"https://cdn.example/002.webp","w":1200,"h":1889}]}"#;

#[test]
fn search_results_map_to_manga() {
    let items = parse_manga_list(SEARCH).unwrap();
    assert_eq!(items.len(), 1);
    let m = &items[0];
    assert_eq!(m.id, MangaId("32807".into()));
    assert_eq!(m.source, "mangadot");
    assert_eq!(m.status, MangaStatus::Ongoing);
    assert_eq!(
        m.cover_url.as_deref(),
        Some("https://mangadot.net/uploads/ea2cc63c.webp")
    );
}

#[test]
fn chapters_filter_language_encode_kind_and_sort() {
    let chapters = parse_chapters(CHAPTERS, &MangaId("9296".into()), "en").unwrap();
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].id, ChapterId("c:616677".into()));
    assert_eq!(chapters[0].number, "1");
    assert_eq!(chapters[0].title.as_deref(), Some("Chapter 1"));
    assert_eq!(chapters[1].id, ChapterId("u:930334".into()));
    assert_eq!(chapters[1].number, "2.5");
    assert_eq!(chapters[1].volume.as_deref(), Some("1"));
    assert_eq!(chapters[1].title, None);
}

#[test]
fn images_path_follows_chapter_kind() {
    assert_eq!(
        images_path(&ChapterId("c:1".into())).unwrap(),
        "/chapters/1/images"
    );
    assert_eq!(
        images_path(&ChapterId("u:2".into())).unwrap(),
        "/uploads/2/images"
    );
    assert!(images_path(&ChapterId("3".into())).is_err());
}

#[test]
fn images_become_absolute_urls() {
    let pages = parse_images(IMAGES).unwrap();
    assert_eq!(
        pages.0,
        vec![
            "https://mangadot.net/chapters/manga_18394/chapter_2/001.webp",
            "https://cdn.example/002.webp"
        ]
    );
    assert_eq!(
        site_url("uploads/a.webp"),
        "https://mangadot.net/uploads/a.webp"
    );
}

#[test]
fn manga_page_reads_open_graph_tags() {
    let html = r#"<html><head><meta property="og:title" content="ONE PIECE &amp; Co"/><meta property="og:description" content="Pirates."><meta property="og:image" content="https://mangadot.net/uploads/op.webp"></head></html>"#;
    let m = parse_manga_page(&MangaId("41".into()), html).unwrap();
    assert_eq!(m.title, "ONE PIECE & Co");
    assert_eq!(m.description, "Pirates.");
    assert_eq!(
        m.cover_url.as_deref(),
        Some("https://mangadot.net/uploads/op.webp")
    );
    assert!(parse_manga_page(&MangaId("1".into()), "<html></html>").is_err());
}

#[test]
fn manga_page_status_comes_from_the_inlined_loader_data() {
    // As served: the stream chunk is a JS string, so quotes arrive escaped.
    let html = r#"<meta property="og:title" content="X"/><script>window.__reactRouterContext.streamController.enqueue("[{"_1":2},"manga",{"_3":4},"id",42,"title","X","status","Completed","photo","/uploads/a.webp","other",{"status":-5}]");</script>"#;
    assert_eq!(loader_status(html).as_deref(), Some("Completed"));
    let m = parse_manga_page(&MangaId("42".into()), html).unwrap();
    assert_eq!(m.status, MangaStatus::Completed);
    assert_eq!(loader_status("<html></html>"), None);
}

#[test]
fn title_matching_is_word_based_and_case_insensitive() {
    assert!(matches("Jujutsu Kaisen", "kaisen"));
    assert!(matches("Jujutsu Kaisen", "JUJUTSU kai"));
    assert!(!matches("Jujutsu Kaisen", "naruto"));
}

#[test]
fn page_headers_come_from_the_host_policy() {
    use mangas_providers::mangadot::MangaDot;
    use mangas_providers::{Session, Source};
    let session = Session {
        cookies: "cf_clearance=abc".into(),
        user_agent: "UA".into(),
        saved_at: None,
    };
    let source = MangaDot::new(Some(session)).unwrap();
    let referer = ("referer".to_string(), "https://mangadot.net/".to_string());
    let on_site = source.page_headers("https://mangadot.net/chapters/x/001.webp");
    assert_eq!(
        on_site,
        vec![
            referer.clone(),
            ("cookie".into(), "cf_clearance=abc".into()),
            ("user-agent".into(), "UA".into()),
        ]
    );
    let lookalike = source.page_headers("https://mangadot.net.evil.io/a.webp");
    assert_eq!(lookalike, vec![referer.clone()]);
    assert_eq!(
        source.page_headers("https://cdn.example/002.webp"),
        vec![referer]
    );
}

#[test]
fn search_endpoint_is_trusted_only_when_results_mostly_match() {
    use mangas_providers::mangadot::honoured;
    let items = parse_manga_list(SEARCH).unwrap();
    assert!(honoured(&items, "konyaku"));
    let latest: Vec<_> = (0..3)
        .flat_map(|_| parse_manga_list(SEARCH).unwrap())
        .collect();
    let mut mixed = latest;
    mixed[0].title = "Naruto".into();
    assert!(!honoured(&mixed, "naruto"));
    assert!(!honoured(&[], "naruto"));
}

#[test]
fn challenge_detection() {
    use mangas_providers::http::is_challenge;
    assert!(is_challenge(403, "challenge", ""));
    assert!(is_challenge(
        403,
        "",
        "<title>Just a moment...</title> cdn-cgi/challenge-platform"
    ));
    assert!(!is_challenge(403, "", "{\"error\":\"forbidden\"}"));
    assert!(!is_challenge(200, "", ""));
}
