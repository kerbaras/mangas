//! ComicInfo labels in a CBZ: the library's series, chapter title, number and volume.

mod export_support;

use export_support::{comic_info, run_labelled, tag_text};
use mangas_core::export::epub::Series;
use mangas_core::export::{Format, Labels};

#[test]
fn cbz_comic_info_names_the_series_chapter_and_volume() {
    let (_root, stats) = run_labelled(Format::Cbz, "kobo-clara", "book.cbz", labels(), false);
    let xml = comic_info(&stats.output);
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let text = |tag: &str| tag_text(&doc, tag);
    assert_eq!(text("Series").as_deref(), Some("Solo Leveling"));
    assert_eq!(text("Title").as_deref(), Some("The Weakest Hunter"));
    assert_eq!(text("Number").as_deref(), Some("2"));
    assert_eq!(text("Volume").as_deref(), Some("1"));
    let order: Vec<&str> = doc
        .root_element()
        .children()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .take(5)
        .collect();
    assert_eq!(order, ["Title", "Series", "Number", "Volume", "PageCount"]);
}

#[test]
fn an_explicit_title_beats_the_chapter_title() {
    let (_root, stats) = run_labelled(Format::Cbz, "kobo-clara", "book.cbz", labels(), true);
    let xml = comic_info(&stats.output);
    let doc = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(tag_text(&doc, "Title").as_deref(), Some("Tom & Jerry"));
    assert_eq!(tag_text(&doc, "Series").as_deref(), Some("Solo Leveling"));
    assert_eq!(tag_text(&doc, "Number").as_deref(), Some("2"));
}

fn labels() -> Labels {
    Labels {
        series: Some("Solo Leveling".into()),
        title: Some("The Weakest Hunter".into()),
        number: Some("2".into()),
        volume: Some("1".into()),
    }
}

#[test]
fn a_volume_export_labels_its_series_and_volume() {
    let series = Series {
        name: "Frieren".into(),
        position: Some("2".into()),
    };
    let filled = Labels::default().or_series(Some(&series));
    assert_eq!(filled.series.as_deref(), Some("Frieren"));
    assert_eq!(filled.volume.as_deref(), Some("2"));
    let known = labels().or_series(Some(&series));
    assert_eq!(known, labels(), "the library's labels win");
}
