//! MangaDex scanlation groups, feed pagination, group preference, data-saver and search filters.

use mangas_core::domain::MangaId;
use mangas_core::library::Settings;
use mangas_providers::mangadex::{
    dedupe_chapters, parse_at_home_response, parse_feed_response, uploads_mirror, MangaDexOptions,
};

const EMINENCE: &str = "91d928a1-7237-4d03-abc3-4b1440cc7d55";

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/mangadex/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn id() -> MangaId {
    MangaId(EMINENCE.into())
}

#[test]
fn feed_maps_scanlation_group_names() {
    let page = parse_feed_response(&fixture("feed_groups_page1.json"), &id(), "en").unwrap();
    let groups: Vec<_> = page
        .chapters
        .iter()
        .map(|c| c.chapter.group.as_deref().unwrap_or("-"))
        .collect();
    assert_eq!(
        groups,
        [
            "The Legacy Guild",
            "Mitsugoshi",
            "KDT SCANS",
            "Mitsugoshi",
            "KDT SCANS",
            "Mitsugoshi"
        ]
    );
    assert_eq!(page.chapters[0].published_at, "2024-06-14T09:17:29+00:00");
}

#[test]
fn feed_pages_report_the_next_offset_until_the_total() {
    let first = parse_feed_response(&fixture("feed_groups_page1.json"), &id(), "en").unwrap();
    let last = parse_feed_response(&fixture("feed_groups_page2.json"), &id(), "en").unwrap();
    assert_eq!(first.next_offset, Some(6), "6 of 12");
    assert_eq!(last.next_offset, None, "offset 6 + 6 reaches total 12");
}

fn both_pages() -> Vec<mangas_providers::mangadex::FeedChapter> {
    ["feed_groups_page1.json", "feed_groups_page2.json"]
        .iter()
        .flat_map(|f| {
            parse_feed_response(&fixture(f), &id(), "en")
                .unwrap()
                .chapters
        })
        .collect()
}

#[test]
fn dedupe_prefers_the_most_recent_upload_without_preferences() {
    let unique = dedupe_chapters(both_pages(), &[]);
    let picked: Vec<_> = unique
        .iter()
        .map(|c| (c.number.as_str(), c.group.as_deref().unwrap_or("-")))
        .collect();
    assert_eq!(
        picked,
        [
            ("1", "The Legacy Guild"),
            ("2.1", "Mitsugoshi"),
            ("2.2", "Mitsugoshi"),
            ("3.1", "KDT SCANS"),
            ("3.2", "Mitsugoshi"),
            ("4.1", "KDT SCANS"),
        ]
    );
}

#[test]
fn dedupe_prefers_configured_groups_in_order() {
    let preferred = vec!["kdt scans".to_string(), "Mitsugoshi".to_string()];
    let unique = dedupe_chapters(both_pages(), &preferred);
    let groups: Vec<_> = unique.iter().map(|c| c.group.as_deref().unwrap()).collect();
    assert_eq!(
        groups, ["KDT SCANS"; 6],
        "KDT uploaded every chapter, so it always wins"
    );

    let preferred = vec!["Mitsugoshi".to_string()];
    let unique = dedupe_chapters(both_pages(), &preferred);
    let groups: Vec<_> = unique.iter().map(|c| c.group.as_deref().unwrap()).collect();
    assert_eq!(
        groups,
        [
            "Mitsugoshi",
            "Mitsugoshi",
            "Mitsugoshi",
            "Mitsugoshi",
            "Mitsugoshi",
            "KDT SCANS"
        ],
        "4.1 has no Mitsugoshi upload, so the most recent other one is kept"
    );
}

#[test]
fn at_home_uses_compressed_pages_with_data_saver() {
    let pages = parse_at_home_response(&fixture("at_home.json"), true).unwrap();
    assert_eq!(pages.0.len(), 30);
    assert_eq!(
        pages.0[0],
        "https://cmdxd98sb0x3yprd.mangadex.network/data-saver/b0d37a668e9a108f695f80fa7260916d/1-c8ac49beb7030dd500baefa205552cb89b7a2e2c6847087665ac2df4d40018f1.jpg"
    );
}

#[test]
fn failed_home_nodes_fall_back_to_uploads() {
    assert_eq!(
        uploads_mirror("https://abc.mangadex.network/data/h/1.png").as_deref(),
        Some("https://uploads.mangadex.org/data/h/1.png")
    );
    assert_eq!(
        uploads_mirror("https://abc.mangadex.network:443/tok3n/data-saver/h/1.jpg").as_deref(),
        Some("https://uploads.mangadex.org/data-saver/h/1.jpg")
    );
    assert_eq!(
        uploads_mirror("https://uploads.mangadex.org/data/h/1.png"),
        None
    );
    assert_eq!(
        uploads_mirror("https://uploads.mangadex.org/covers/m/c.jpg"),
        None
    );
    assert_eq!(uploads_mirror("not a url"), None);
}

#[test]
fn options_default_to_safe_and_suggestive_without_data_saver() {
    let options = MangaDexOptions::from_settings(&Settings::default());
    assert_eq!(options.ratings(), ["safe", "suggestive"]);
    assert!(!options.data_saver);
    assert!(options.preferred_groups.is_empty());
}

#[test]
fn options_are_read_from_provider_settings() {
    let settings = Settings {
        provider_options: [(
            "mangadex".to_string(),
            serde_json::json!({ "content_ratings": ["erotica", "SAFE", "bogus"], "data_saver": true }),
        )]
        .into(),
        ..Settings::default()
    };
    let options = MangaDexOptions::from_settings(&settings);
    assert_eq!(
        options.ratings(),
        ["safe", "erotica"],
        "known ones, MangaDex order"
    );
    assert!(options.data_saver);

    let broken = Settings {
        provider_options: [("mangadex".to_string(), serde_json::json!("nope"))].into(),
        ..Settings::default()
    };
    assert_eq!(
        MangaDexOptions::from_settings(&broken),
        MangaDexOptions::default()
    );
}
