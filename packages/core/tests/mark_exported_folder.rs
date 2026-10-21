//! #310: `library::mark_exported_folder` marks the chapters stored at any spelling of a folder
//! and says which they were; a folder no chapter is stored at marks nothing.

use std::path::Path;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{self, ExportedChapter};

#[test]
fn the_chapter_stored_at_a_spelling_is_marked_and_named() {
    let lib = library::open_in_memory().unwrap();
    let id = MangaId("m".into());
    let manga = Manga {
        id: id.clone(),
        source: "mangadex".into(),
        title: "Kaiju No. 8".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("c1".into()),
        manga_id: id.clone(),
        language: "en".into(),
        number: "12.5".into(),
        ..Default::default()
    };
    library::upsert_chapters(&lib, "mangadex", &id, "en", std::slice::from_ref(&chapter)).unwrap();
    library::mark_downloaded(&lib, "mangadex", &chapter.id, Path::new("/data/m/12.5")).unwrap();

    let missing = [Path::new("/elsewhere/12.5")];
    assert_eq!(library::mark_exported_folder(&lib, &missing).unwrap(), []);
    let chapters = library::list_chapters(&lib, "mangadex", &id, None).unwrap();
    assert!(chapters[0].exported_at.is_none());

    let spellings = [Path::new("/elsewhere/12.5"), Path::new("/data/m/12.5/")];
    let marked = library::mark_exported_folder(&lib, &spellings).unwrap();
    let expected = ExportedChapter {
        manga: "Kaiju No. 8".into(),
        chapter: "12.5".into(),
    };
    assert_eq!(marked, [expected]);
    let chapters = library::list_chapters(&lib, "mangadex", &id, None).unwrap();
    assert!(chapters[0].exported_at.is_some());
}

#[test]
fn every_chapter_stored_at_the_folder_is_named_once() {
    let lib = library::open_in_memory().unwrap();
    let id = MangaId("m".into());
    let manga = Manga {
        id: id.clone(),
        source: "mangadex".into(),
        title: "M".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapters: Vec<Chapter> = ["1", "1.5"]
        .iter()
        .map(|n| Chapter {
            id: ChapterId(format!("c{n}")),
            manga_id: id.clone(),
            language: "en".into(),
            number: (*n).into(),
            ..Default::default()
        })
        .collect();
    library::upsert_chapters(&lib, "mangadex", &id, "en", &chapters).unwrap();
    for c in &chapters {
        library::mark_downloaded(&lib, "mangadex", &c.id, Path::new("/data/m/1")).unwrap();
    }
    let spellings = [Path::new("/data/m/1"), Path::new("/data/m/1/")];
    let marked = library::mark_exported_folder(&lib, &spellings).unwrap();
    let mut numbers: Vec<&str> = marked.iter().map(|c| c.chapter.as_str()).collect();
    numbers.sort();
    assert_eq!(numbers, ["1", "1.5"]);
}
