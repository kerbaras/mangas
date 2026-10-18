use mangas_core::domain::{ChapterId, Error, MangaId, MangaStatus};
use mangas_providers::mangadot::{
    images_path, matches, parse_chapters, parse_images, parse_manga_detail, parse_manga_list,
    site_url,
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

/// `/api/manga/115` as recorded live on the PM tunnel (#94), trimmed.
const DETAIL: &str = r#"{"manga":{"id":115,"title":"BERSERK","genres":["Action","Seinen"],"status":"Ongoing","photo":"/uploads/cover_eaee0cbaeae8d124.jpg","date_added":"2026-02-19 13:01:59+00","description":"His name is Guts, the Black Swordsman.","banner_image":"/uploads/banners/821b5570ba731094cbbec2a9.webp","is_adult":false,"chapter_count":434,"latest_chapter_number":"386.00","authors":"[\"MIURA Kentaro\"]","year":1989,"avg_rating":9.4,"alt_titles":["Berserk Max"],"tags":[{"category":"Audience Demographics","is_adult":false,"tags":[{"name":"Seinen","weight":"core","is_adult":false}]}]}}"#;

#[test]
fn manga_detail_maps_the_api_record() {
    let m = parse_manga_detail(&MangaId("115".into()), DETAIL).unwrap();
    assert_eq!(m.id, MangaId("115".into()));
    assert_eq!(m.source, "mangadot");
    assert_eq!(m.title, "BERSERK");
    assert_eq!(m.description, "His name is Guts, the Black Swordsman.");
    assert_eq!(
        m.cover_url.as_deref(),
        Some("https://mangadot.net/uploads/cover_eaee0cbaeae8d124.jpg")
    );
    assert_eq!(m.status, MangaStatus::Ongoing);
}

#[test]
fn manga_detail_without_a_record_is_not_found() {
    let id = MangaId("1".into());
    assert!(matches!(
        parse_manga_detail(&id, r#"{"manga":null}"#),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        parse_manga_detail(&id, "{}"),
        Err(Error::NotFound(_))
    ));
    assert!(parse_manga_detail(&id, "<html>").is_err());
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
