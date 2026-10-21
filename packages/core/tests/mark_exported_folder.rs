//! #310: `library::mark_exported_folder` marks the one chapter stored at any spelling of a folder
//! and says which it was; a folder no chapter is stored at marks nothing.

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
    assert_eq!(library::mark_exported_folder(&lib, &missing).unwrap(), None);
    let chapters = library::list_chapters(&lib, "mangadex", &id, None).unwrap();
    assert!(chapters[0].exported_at.is_none());

    let spellings = [Path::new("/elsewhere/12.5"), Path::new("/data/m/12.5/")];
    let marked = library::mark_exported_folder(&lib, &spellings).unwrap();
    let expected = ExportedChapter {
        manga: "Kaiju No. 8".into(),
        chapter: "12.5".into(),
    };
    assert_eq!(marked, Some(expected));
    let chapters = library::list_chapters(&lib, "mangadex", &id, None).unwrap();
    assert!(chapters[0].exported_at.is_some());
}
