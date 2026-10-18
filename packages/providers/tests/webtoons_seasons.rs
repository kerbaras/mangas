use mangas_providers::webtoons::{parse_episode_page, to_chapters, Title};

#[test]
fn season_titles_get_display_labels_and_keep_the_absolute_episode_number() {
    let title = Title::original("en", 95);
    let body = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/webtoons/season_episodes.json"
    ))
    .unwrap();
    let page = parse_episode_page(&body, &title).unwrap();
    let chapters = to_chapters(&title.id(), &title, vec![page]);
    let got: Vec<_> = chapters
        .iter()
        .map(|c| (c.number.as_str(), c.title.as_deref().unwrap_or("")))
        .collect();
    assert_eq!(
        got,
        [
            ("1", "S1 Ep. 0"),
            ("2", "S1 Ep. 1 - 1F.Headon's Floor"),
            ("80", "S2 Ep. 0"),
            ("81", "S2 Ep. 1"),
            ("419", "S3 Ep. 1"),
            ("653", "S3 Ep. 235 (Season 3 Finale)"),
        ]
    );
}
