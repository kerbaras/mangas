//! Name -> chapter parsing, slugs, natural order and the import report summary.

use std::cmp::Ordering;
use std::path::PathBuf;

use mangas_core::domain::{Manga, MangaId, MangaStatus};
use mangas_core::library::import::{
    natural_cmp, parse_chapter_name, slug, ImportReport, ImportedChapter, ParsedName, Skipped,
};

fn parsed(number: Option<&str>, volume: Option<&str>) -> ParsedName {
    ParsedName {
        number: number.map(str::to_string),
        volume: volume.map(str::to_string),
    }
}

#[test]
fn chapter_keywords_win_over_other_numbers() {
    let cases = [
        ("Chapter 12", parsed(Some("12"), None)),
        ("ch.012.5 - The Return", parsed(Some("12.5"), None)),
        ("c003", parsed(Some("3"), None)),
        ("Solo Leveling S2 Ep 7", parsed(Some("7"), None)),
        ("#45 Finale", parsed(Some("45"), None)),
        ("Vol.2 Ch.15", parsed(Some("15"), Some("2"))),
        ("v03 c021 [Group]", parsed(Some("21"), Some("3"))),
        ("Chapter12v2", parsed(Some("12"), None)),
        ("Chapter 0", parsed(Some("0"), None)),
    ];
    for (name, want) in cases {
        assert_eq!(parse_chapter_name(name), want, "{name}");
    }
}

#[test]
fn falls_back_to_the_last_number_then_the_volume() {
    assert_eq!(
        parse_chapter_name("One Piece 1001"),
        parsed(Some("1001"), None)
    );
    assert_eq!(parse_chapter_name("012"), parsed(Some("12"), None));
    assert_eq!(
        parse_chapter_name("Berserk 2 - 05.cbz"),
        parsed(Some("5"), None)
    );
    assert_eq!(
        parse_chapter_name("Volume 03"),
        parsed(Some("3"), Some("3"))
    );
    assert_eq!(parse_chapter_name("Extras"), parsed(None, None));
    assert_eq!(parse_chapter_name("Oneshot.zip"), parsed(None, None));
}

#[test]
fn folder_names_keep_their_dots() {
    assert_eq!(parse_chapter_name("12.5"), parsed(Some("12.5"), None));
    assert_eq!(parse_chapter_name("12.5.cbz"), parsed(Some("12.5"), None));
}

#[test]
fn slugs_are_ascii_and_stable() {
    assert_eq!(slug("One Piece!"), "one-piece");
    assert_eq!(slug("  Tower of God (Season 2) "), "tower-of-god-season-2");
    let jp = slug("ワンピース");
    assert!(
        jp.starts_with("local-") && jp.len() == "local-".len() + 12,
        "{jp}"
    );
    assert_eq!(jp, slug("ワンピース"));
}

#[test]
fn natural_order_compares_digit_runs_by_value() {
    let mut names = vec![
        "page10.jpg",
        "page2.jpg",
        "Page1.jpg",
        "page02b.jpg",
        "cover.jpg",
    ];
    names.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(
        names,
        [
            "cover.jpg",
            "Page1.jpg",
            "page2.jpg",
            "page02b.jpg",
            "page10.jpg"
        ]
    );
    assert_eq!(natural_cmp("007", "7"), Ordering::Equal);
    assert_eq!(natural_cmp("Chapter 9", "Chapter 10"), Ordering::Less);
}

fn report(imported: &[(&str, usize)], skipped: usize) -> ImportReport {
    ImportReport {
        manga: Manga {
            id: MangaId("berserk".into()),
            source: "local".into(),
            title: "Berserk".into(),
            description: String::new(),
            cover_url: None,
            status: MangaStatus::Unknown,
            ..Default::default()
        },
        imported: imported
            .iter()
            .map(|(number, pages)| ImportedChapter {
                number: number.to_string(),
                title: None,
                pages: *pages,
                path: PathBuf::from(number),
            })
            .collect(),
        skipped: (0..skipped)
            .map(|i| Skipped::new(&i.to_string(), "no images"))
            .collect(),
    }
}

#[test]
fn report_summary_counts_chapters_pages_and_skips() {
    let r = report(&[("1", 10), ("2", 12), ("", 3)], 2);
    assert_eq!(r.pages(), 25);
    assert_eq!(r.unnumbered(), 1);
    assert_eq!(
        r.summary(),
        "Imported 3 chapters (25 pages) of Berserk; 1 without a number; 2 skipped"
    );
    assert_eq!(
        report(&[("1", 4)], 0).summary(),
        "Imported 1 chapter (4 pages) of Berserk"
    );
    assert_eq!(
        report(&[("1", 1)], 0).summary(),
        "Imported 1 chapter (1 page) of Berserk"
    );
    assert_eq!(
        report(&[], 3).summary(),
        "Nothing new in Berserk; 3 skipped"
    );
}
