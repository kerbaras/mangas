//! Volume planning: grouping, titles, file names and metadata (pure).

use mangas_core::domain::{Chapter, ChapterId, Manga};
use mangas_core::export::kindle::device;
use mangas_core::export::volume::{
    book_meta, chapter_label, file_name, find_volume, group, plan, request, Grouping, Volume,
};
use mangas_core::smartcut::Preset;

fn ch(number: &str, volume: Option<&str>) -> Chapter {
    Chapter {
        id: ChapterId(format!("c{number}")),
        number: number.into(),
        volume: volume.map(Into::into),
        language: "en".into(),
        local_path: Some(format!("/lib/ch{number}")),
        ..Default::default()
    }
}

fn manga() -> Manga {
    Manga {
        title: "Blue Period".into(),
        authors: vec!["Tsubasa Yamaguchi".into()],
        artists: vec!["Tsubasa Yamaguchi".into(), "Helper".into()],
        ..Default::default()
    }
}

fn numbers(v: &Volume) -> Vec<&str> {
    v.chapters.iter().map(|c| c.number.as_str()).collect()
}

#[test]
fn groups_by_provider_volume_in_volume_order_with_loose_chapters_last() {
    let chapters = vec![
        ch("1", Some("1")),
        ch("2", Some(" 1 ")),
        ch("3", Some("2")),
        ch("4", None),
        ch("5", Some("")),
    ];
    let volumes = group(&chapters, Grouping::ByVolume);
    assert_eq!(volumes.len(), 3);
    assert_eq!(volumes[0].number.as_deref(), Some("1"));
    assert_eq!(numbers(&volumes[0]), ["1", "2"]);
    assert_eq!(volumes[1].number.as_deref(), Some("2"));
    assert_eq!(volumes[2].number, None);
    assert_eq!(numbers(&volumes[2]), ["4", "5"]);
}

#[test]
fn volumes_sort_numerically_not_by_text() {
    let chapters = vec![ch("100", Some("10")), ch("1", Some("2"))];
    let volumes = group(&chapters, Grouping::ByVolume);
    let order: Vec<_> = volumes.iter().map(|v| v.number.clone().unwrap()).collect();
    assert_eq!(order, ["2", "10"]);
}

#[test]
fn every_n_numbers_books_from_one_and_single_keeps_a_shared_volume() {
    let chapters: Vec<_> = (1..=5).map(|n| ch(&n.to_string(), Some("1"))).collect();
    let every = group(&chapters, Grouping::Every(2));
    assert_eq!(every.len(), 3);
    assert_eq!(every[2].number.as_deref(), Some("3"));
    assert_eq!(numbers(&every[2]), ["5"]);
    let single = group(&chapters, Grouping::Single);
    assert_eq!(single[0].number.as_deref(), Some("1"));
    let mixed = group(&[ch("1", Some("1")), ch("9", Some("2"))], Grouping::Single);
    assert_eq!(mixed[0].number, None);
    assert!(group(&[], Grouping::ByVolume).is_empty());
    assert_eq!(
        group(&chapters, Grouping::Every(0)).len(),
        5,
        "0 counts as 1"
    );
}

#[test]
fn plan_names_books_and_skips_chapters_not_on_disk() {
    let mut missing = ch("3", Some("2"));
    missing.local_path = None;
    let chapters = vec![
        ch("1", Some("1")),
        ch("2", Some("1")),
        missing,
        ch("4", None),
    ];
    let plans = plan(&manga(), &chapters, Grouping::ByVolume, None, None);
    let names: Vec<_> = plans.iter().map(|p| p.file_name.as_str()).collect();
    assert_eq!(names, ["Blue Period v1.epub", "Blue Period c4.epub"]);
    assert_eq!(plans[0].title, "Blue Period v1");
    assert_eq!(find_volume(&plans, "01").map(|p| p.chapters.len()), Some(2));
    assert!(
        find_volume(&plans, "2").is_none(),
        "volume 2 has nothing downloaded"
    );

    let custom = plan(
        &manga(),
        &chapters,
        Grouping::Single,
        Some("{series} {first}-{last}"),
        None,
    );
    assert_eq!(custom[0].title, "Blue Period 1-4");
    let titled = plan(&manga(), &chapters, Grouping::Single, None, Some("Best of"));
    assert_eq!(titled[0].file_name, "Best of.epub");
    let range = plan(
        &manga(),
        &chapters[..2],
        Grouping::Every(1),
        None,
        Some("Ignored"),
    );
    assert_eq!(
        range[1].title, "Blue Period v2",
        "a title only names a single book"
    );
}

#[test]
fn file_names_are_portable() {
    assert_eq!(file_name("Re:Zero / Part 1?"), "Re Zero Part 1.epub");
    assert_eq!(file_name("Dr. Stone..."), "Dr. Stone.epub");
    assert_eq!(file_name(" :: "), "Manga.epub");
    assert_eq!(file_name("CON"), "CON (manga).epub");
    assert_eq!(file_name("com3.v1"), "com3.v1 (manga).epub");
    assert_eq!(file_name("Console"), "Console.epub");
}

#[test]
fn metadata_comes_from_the_manga_details() {
    let mut first = ch("12", Some("3"));
    first.title = Some("Golden hour".into());
    let volume = Volume {
        number: Some("3".into()),
        chapters: vec![first, ch("13", Some("3"))],
    };
    let meta = book_meta(&manga(), &volume, None);
    assert_eq!(meta.authors, ["Tsubasa Yamaguchi", "Helper"]);
    assert_eq!(meta.language.as_deref(), Some("en"));
    let series = meta.series.unwrap();
    assert_eq!(
        (series.name.as_str(), series.position.as_deref()),
        ("Blue Period", Some("3"))
    );
    assert_eq!(
        meta.chapter_titles,
        ["Chapter 12: Golden hour", "Chapter 13"]
    );
    let mut oneshot = ch("", None);
    oneshot.title = Some("Oneshot".into());
    assert_eq!(chapter_label(&oneshot), "Oneshot");
}

#[test]
fn request_writes_into_the_output_folder_with_the_planned_name() {
    let plans = plan(
        &manga(),
        &[ch("1", Some("1"))],
        Grouping::ByVolume,
        None,
        None,
    );
    let dev = device("kindle-paperwhite12").unwrap();
    let req = request(
        &manga(),
        &plans[0],
        dev,
        Preset::Balanced.options(),
        "/out".as_ref(),
        None,
    );
    assert_eq!(req.output, std::path::Path::new("/out/Blue Period v1.epub"));
    assert_eq!(req.chapter_dirs, [std::path::PathBuf::from("/lib/ch1")]);
    assert_eq!(req.title, "Blue Period v1");
}

#[test]
fn find_volume_prefers_the_exact_label_over_a_numeric_match() {
    let chapters = [ch("1", Some("03")), ch("2", Some("3")), ch("3", Some("4"))];
    let plans = plan(&manga(), &chapters, Grouping::ByVolume, None, None);
    let found = |n: &str| find_volume(&plans, n).map(|p| p.chapters[0].number.clone());
    assert_eq!(found("3").as_deref(), Some("2"));
    assert_eq!(found("03").as_deref(), Some("1"));
    assert_eq!(found("4.0").as_deref(), Some("3"));
}
