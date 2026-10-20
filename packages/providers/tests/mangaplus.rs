//! MANGA Plus: protobuf replies (title list, details, chapters, errors) mapped over recorded
//! fixtures.

use mangas_core::domain::{Availability, MangaId, MangaStatus};
use mangas_providers::mangaplus::{
    chapter_number, language_code, parse_chapters, parse_search, parse_title,
};

fn fixture(name: &str) -> Vec<u8> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mangaplus/");
    std::fs::read(format!("{dir}{name}")).unwrap()
}

fn en() -> Vec<String> {
    vec!["en".to_string()]
}

#[test]
fn search_filters_the_title_list_by_name_and_language() {
    let body = fixture("contract/all_titles.pb");
    let found = parse_search(&body, "BORUTO", &en(), 10).unwrap();
    let titles: Vec<_> = found.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(
        titles,
        ["Boruto: Two Blue Vortex", "Boruto: Naruto Next Generations"]
    );
    assert_eq!(found[0].id, MangaId("100269".into()));
    assert_eq!(found[0].authors, ["Masashi Kishimoto", "Mikio Ikemoto"]);
    assert!(found[0]
        .cover_url
        .as_deref()
        .unwrap()
        .starts_with("https://"));
    let spanish = parse_search(&body, "boruto", &["es".to_string()], 10).unwrap();
    assert!(!spanish.is_empty() && spanish.iter().all(|m| m.id != found[0].id));
    assert!(parse_search(&body, "no such title", &en(), 10)
        .unwrap()
        .is_empty());
}

#[test]
fn title_details_map_to_a_manga() {
    let id = MangaId("100269".into());
    let manga = parse_title(&fixture("contract/title_detail.pb"), &id).unwrap();
    assert_eq!(manga.title, "Boruto: Two Blue Vortex");
    assert_eq!(manga.source, "mangaplus");
    assert_eq!(manga.status, MangaStatus::Ongoing);
    assert!(
        manga.description.starts_with("With everyone's memories"),
        "{}",
        manga.description
    );
    assert_eq!(manga.tags, ["Battle / Action", "Sci-Fi / Fantasy"]);
}

#[test]
fn chapters_are_the_free_ones_in_the_title_language() {
    let id = MangaId("100269".into());
    let body = fixture("contract/title_detail.pb");
    let chapters = parse_chapters(&body, &id, "en").unwrap();
    let numbers: Vec<_> = chapters.iter().map(|c| c.number.as_str()).collect();
    assert_eq!(numbers, ["1", "2", "3", "35", "36", "37"]);
    assert_eq!(chapters[0].id.0, "1018835");
    assert_eq!(chapters[0].title.as_deref(), Some("1: Boruto"));
    assert!(chapters
        .iter()
        .all(|c| c.availability == Availability::Available));
    assert!(parse_chapters(&body, &id, "es").unwrap().is_empty());
}

#[test]
fn chapter_numbers_and_languages() {
    assert_eq!(chapter_number("#012"), "12");
    assert_eq!(chapter_number("#1100.5"), "1100.5");
    assert_eq!(chapter_number("#000"), "0");
    assert_eq!(chapter_number("ex"), "");
    assert_eq!(chapter_number("One-shot"), "");
    assert_eq!(language_code(4), Some("pt-br"));
    assert_eq!(language_code(8), None);
    assert_eq!(language_code(-1), None);
}

#[test]
fn an_error_popup_is_the_error() {
    let err = parse_search(&fixture("banned.pb"), "boruto", &en(), 10).unwrap_err();
    assert!(err.to_string().contains("Account Banned"), "{err}");
}

#[test]
fn status_comes_from_the_release_schedule() {
    let status = |name: &str, id: &str| {
        let manga = parse_title(&fixture(name), &MangaId(id.into())).unwrap();
        manga.status
    };
    assert_eq!(
        status("contract/title_detail.pb", "100269"),
        MangaStatus::Ongoing
    );
    assert_eq!(status("completed.pb", "100187"), MangaStatus::Completed);
    assert_eq!(status("one_shot.pb", "100188"), MangaStatus::Completed);
    assert_eq!(status("unscheduled.pb", "100512"), MangaStatus::Unknown);
}

#[test]
fn search_results_have_no_status() {
    let found = parse_search(&fixture("contract/all_titles.pb"), "boruto", &en(), 10).unwrap();
    assert!(found.iter().all(|m| m.status == MangaStatus::Unknown));
}
