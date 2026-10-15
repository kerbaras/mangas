use mangas_core::domain::{Chapter, ChapterId, MangaId};
use std::path::Path;

use mangas_core::download::{cbz_path, chapter_dir, parse_selection, select_chapters, write_cbz};

fn chapter(number: &str) -> Chapter {
    Chapter {
        id: ChapterId(format!("ch-{number}")),
        manga_id: MangaId("m".into()),
        title: None,
        language: "en".into(),
        volume: None,
        number: number.into(),
        local_path: None,
    }
}

#[test]
fn cbz_stores_pages_in_order() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "0002.jpg",
        "0001.png",
        "0003.png.part",
        "0010.jpg",
        "ComicInfo.xml",
        ".DS_Store",
    ] {
        std::fs::write(dir.path().join(name), name).unwrap();
    }
    let out = dir.path().join("out").join("ch.cbz");
    assert_eq!(write_cbz(dir.path(), &out).unwrap(), 3);
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().unwrap().into_owned())
        .collect();
    assert_eq!(names, ["0001.png", "0002.jpg", "0010.jpg"]);
    assert_eq!(
        archive.by_index(0).unwrap().compression(),
        zip::CompressionMethod::Stored
    );
}

#[test]
fn selection_parses_numbers_and_ranges() {
    assert_eq!(
        parse_selection("1-5,7").unwrap(),
        vec![(1.0, 5.0), (7.0, 7.0)]
    );
    assert_eq!(
        parse_selection(" 2.5 , 10-12 ").unwrap(),
        vec![(2.5, 2.5), (10.0, 12.0)]
    );
    assert!(parse_selection("5-1").is_err());
    assert!(parse_selection("abc").is_err());
    assert!(parse_selection("").is_err() || parse_selection("").unwrap().is_empty());
}

#[test]
fn select_chapters_by_published_number() {
    let all: Vec<Chapter> = ["1", "2", "2.5", "3", "7", "Oneshot"]
        .iter()
        .map(|n| chapter(n))
        .collect();
    let picked: Vec<String> = select_chapters(&all, "1-2,7")
        .unwrap()
        .into_iter()
        .map(|c| c.number)
        .collect();
    assert_eq!(picked, ["1", "2", "7"]);
    assert!(select_chapters(&all, "x").is_err());
    assert!(select_chapters(&all, "").is_err());
}

#[test]
fn cbz_path_keeps_decimal_chapter_names() {
    assert_eq!(cbz_path(Path::new("/x/m/2.5")), Path::new("/x/m/2.5.cbz"));
    assert_eq!(cbz_path(Path::new("/x/m/12")), Path::new("/x/m/12.cbz"));
}

#[test]
fn unnumbered_chapters_get_their_own_directory() {
    let root = Path::new("/dl");
    assert_eq!(
        chapter_dir(root, "mangadex", &chapter("2.5")),
        Path::new("/dl/mangadex/m/2.5")
    );
    let a = chapter_dir(root, "mangadex", &chapter(""));
    let mut other = chapter("");
    other.id = mangas_core::domain::ChapterId("other".into());
    assert_eq!(a, Path::new("/dl/mangadex/m/ch-"));
    assert_ne!(a, chapter_dir(root, "mangadex", &other));
}
