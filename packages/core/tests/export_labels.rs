//! ComicInfo labels looked up from the library by the exported chapters' folders.

use std::path::Path;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::export::Labels;
use mangas_core::library::{
    chapter_titles, export_labels, mark_downloaded, open_in_memory, upsert_chapters, upsert_manga,
    Library,
};

fn follow(lib: &Library, id: &str, title: &str, chapters: &[(&str, &str, Option<&str>)]) {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: title.into(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    let list: Vec<Chapter> = chapters
        .iter()
        .map(|(cid, number, volume)| Chapter {
            id: ChapterId((*cid).into()),
            manga_id: manga.id.clone(),
            title: Some(format!("Chapter title {number}")),
            language: "en".into(),
            volume: volume.map(str::to_owned),
            number: (*number).into(),
            ..Default::default()
        })
        .collect();
    upsert_chapters(lib, "mangadex", &manga.id, "en", &list).unwrap();
    for (cid, _, _) in chapters {
        let dir = format!("/lib/{id}/{cid}");
        mark_downloaded(lib, "mangadex", &ChapterId((*cid).into()), Path::new(&dir)).unwrap();
    }
}

fn library() -> Library {
    let lib = open_in_memory().unwrap();
    follow(
        &lib,
        "a",
        "Solo Leveling",
        &[
            ("a1", "1", Some("1")),
            ("a2", "2", Some("1")),
            ("a3", "3", None),
        ],
    );
    follow(&lib, "b", "Berserk", &[("b1", "1", Some("1"))]);
    follow(&lib, "c", "Solo Leveling", &[("c1", "1", Some("1"))]);
    lib
}

fn labels(lib: &Library, dirs: &[&str], chapters: usize) -> Labels {
    let paths: Vec<&Path> = dirs.iter().map(Path::new).collect();
    export_labels(lib, &paths, chapters).unwrap()
}

#[test]
fn one_chapter_gets_series_title_number_and_volume() {
    let lib = library();
    let got = labels(&lib, &["/lib/a/a2/"], 1);
    assert_eq!(
        got,
        Labels {
            series: Some("Solo Leveling".into()),
            title: Some("Chapter title 2".into()),
            number: Some("2".into()),
            volume: Some("1".into()),
        }
    );
}

#[test]
fn several_chapters_keep_only_what_they_share() {
    let lib = library();
    let same_volume = labels(&lib, &["/lib/a/a1", "/lib/a/a2"], 2);
    assert_eq!(same_volume.series.as_deref(), Some("Solo Leveling"));
    assert_eq!(same_volume.volume.as_deref(), Some("1"));
    assert_eq!((same_volume.title, same_volume.number), (None, None));
    let mixed_volume = labels(&lib, &["/lib/a/a2", "/lib/a/a3"], 2);
    assert_eq!(mixed_volume.series.as_deref(), Some("Solo Leveling"));
    assert_eq!(mixed_volume.volume, None);
    assert_eq!(
        labels(&lib, &["/lib/a/a1", "/lib/b/b1"], 2),
        Labels::default()
    );
}

#[test]
fn duplicate_spellings_count_once_and_unknown_folders_drop_the_labels() {
    let lib = library();
    let twice = labels(&lib, &["/lib/a/a2", "/lib/a/a2/"], 1);
    assert_eq!(twice.number.as_deref(), Some("2"));
    assert_eq!(
        labels(&lib, &["/lib/a/a2", "/elsewhere/x"], 2),
        Labels::default()
    );
    assert_eq!(labels(&lib, &["/elsewhere/x"], 1), Labels::default());
}

#[test]
fn chapter_titles_follow_the_folder_order_or_give_up() {
    let lib = library();
    let dirs = |ds: &[&'static str]| -> Vec<Vec<&'static Path>> {
        ds.iter()
            .map(|d| vec![Path::new("/nowhere"), Path::new(*d)])
            .collect()
    };
    assert_eq!(
        chapter_titles(&lib, &dirs(&["/lib/a/a2/", "/lib/a/a1"])).unwrap(),
        ["Chapter 2: Chapter title 2", "Chapter 1: Chapter title 1"]
    );
    assert!(chapter_titles(&lib, &dirs(&["/lib/a/a1", "/elsewhere/x"]))
        .unwrap()
        .is_empty());
}
