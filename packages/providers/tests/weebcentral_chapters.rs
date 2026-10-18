//! WeebCentral chapter lists: labels, numbering and gap filling.

use mangas_core::domain::{ChapterId, MangaId};
use mangas_providers::weebcentral::{
    fill_gaps, number_chapters, parse_chapters, parse_label, Label,
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
fn prologues_come_first_numbered_before_chapter_one() {
    let id = MangaId("01J76XY7EF75DJNQCV04HTPDZK".into());
    let chapters = parse_chapters(&fixture("chapters_prologue.html"), &id).unwrap();
    let expected = [
        ("0.3", Some("Prologue 1")),
        ("0.7", Some("Prologue 16")),
        ("1", None),
        ("2", None),
        ("350.5", None),
    ];
    let expected: Vec<_> = expected
        .iter()
        .map(|(n, t)| (n.to_string(), t.map(str::to_string)))
        .collect();
    assert_eq!(summary(&chapters), expected);
}

#[test]
fn gaps_are_spread_between_their_neighbours() {
    let numbers = |v: &[Option<&str>]| v.iter().map(|n| n.map(str::to_string)).collect();
    let filled = fill_gaps(numbers(&[None, Some("0"), None, None, Some("1.5"), None]));
    assert_eq!(filled, ["-0.5", "0", "0.5", "1", "1.5", "2"]);
    assert_eq!(fill_gaps(numbers(&[None, None])), ["0.3", "0.7"]);
    let long = fill_gaps(numbers(&[&[None; 16][..], &[Some("1")]].concat()));
    assert_eq!((long[0].as_str(), long[15].as_str()), ("0.06", "0.94"));
    // Close neighbours get enough decimals to stay apart.
    let close = fill_gaps(numbers(&[Some("1"), None, Some("1.001")]));
    assert_eq!(close, ["1", "1.0005", "1.001"]);
    // Chapter selections cannot name negative numbers.
    let first = fill_gaps(numbers(&[None, Some("0.5")]))[0]
        .parse::<f64>()
        .unwrap();
    assert!(first > 0.0 && first < 0.5, "{first}");
    let repeated = number_chapters(
        vec![
            (ChapterId("C".into()), "S2 - Chapter 1".to_string()),
            (ChapterId("B".into()), "S1 - Chapter 2".to_string()),
            (ChapterId("C".into()), "S2 - Chapter 1".to_string()),
            (ChapterId("A".into()), "S1 - Chapter 1".to_string()),
        ],
        &MangaId("1".into()),
    );
    let ids: Vec<_> = repeated
        .iter()
        .map(|c| (c.id.0.as_str(), c.number.as_str()))
        .collect();
    assert_eq!(ids, [("A", "1"), ("B", "2"), ("C", "3")]);
    // Out of order neighbours: the run still follows the one before it.
    assert_eq!(
        fill_gaps(numbers(&[Some("5"), None, Some("2")])),
        ["5", "5.5", "2"]
    );
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
