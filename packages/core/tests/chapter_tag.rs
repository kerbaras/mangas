//! Chapter tags and TOC labels. The fixture is shared with the desktop's `season-tag.ts`.

use mangas_core::domain::{chapter_tag, season_tag, toc_label, Chapter};

const FIXTURE: &str = include_str!("fixtures/season_tags.json");

#[derive(serde::Deserialize)]
struct Case {
    title: String,
    tag: Option<String>,
}

fn ch(number: &str, title: Option<&str>) -> Chapter {
    Chapter {
        number: number.into(),
        title: title.map(Into::into),
        ..Default::default()
    }
}

#[test]
fn season_tags_match_the_shared_fixture() {
    let cases: Vec<Case> = serde_json::from_str(FIXTURE).unwrap();
    for c in cases {
        assert_eq!(
            season_tag(&c.title),
            c.tag.as_deref(),
            "title {:?}",
            c.title
        );
    }
}

#[test]
fn chapter_tag_falls_back_to_the_number() {
    assert_eq!(chapter_tag(&ch("80", Some("S2 Ep. 0"))), "S2 Ep. 0");
    assert_eq!(chapter_tag(&ch("79", Some("S2 Prologue"))), "S2 Prologue");
    assert_eq!(chapter_tag(&ch(" 12 ", Some("The Return"))), "c12");
    assert_eq!(chapter_tag(&ch("3", None)), "c3");
}

#[test]
fn toc_label_shows_season_titles_without_the_absolute_number() {
    assert_eq!(toc_label("80", Some("S2 Ep. 0")), "S2 Ep. 0");
    assert_eq!(
        toc_label("2", Some("S1 Ep. 1 - 1F.Headon's Floor")),
        "S1 Ep. 1 - 1F.Headon's Floor"
    );
    assert_eq!(
        toc_label("12", Some("The Return")),
        "Chapter 12: The Return"
    );
    assert_eq!(toc_label("12", Some("  ")), "Chapter 12");
    assert_eq!(toc_label("", Some("Oneshot")), "Oneshot");
    assert_eq!(toc_label("", None), "Chapter");
}
