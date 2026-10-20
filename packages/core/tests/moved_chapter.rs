//! #239: a chapter folder copied out of the downloads folder is matched back to the library by
//! its `<source>/<manga id>/<chapter>` names, and never titled after the manga id.

use std::path::{Path, PathBuf};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    mark_downloaded, open_in_memory, series_folder_name, stored_original, upsert_chapters,
    upsert_manga, Library,
};

fn follow(lib: &Library, source: &str, id: &str, title: &str) -> PathBuf {
    let manga = Manga {
        id: MangaId(id.into()),
        source: source.into(),
        title: title.into(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId(format!("{id}-1")),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    upsert_chapters(lib, source, &manga.id, "en", std::slice::from_ref(&chapter)).unwrap();
    let dir = PathBuf::from(format!("/data/chapters/{source}/{id}/1"));
    mark_downloaded(lib, source, &chapter.id, &dir).unwrap();
    dir
}

fn original(lib: &Library, dir: &str) -> Vec<PathBuf> {
    stored_original(lib, &[Path::new(dir)], &["mangadex", "mangadot", "local"]).unwrap()
}

#[test]
fn a_copied_chapter_folder_finds_the_stored_one() {
    let lib = open_in_memory().unwrap();
    let stored = follow(&lib, "mangadex", "m-uuid", "Kaiju No. 8");
    assert_eq!(
        original(&lib, "/elsewhere/mangadex/m-uuid/1/"),
        vec![stored.clone()]
    );
    assert_eq!(original(&lib, "/old-data/m-uuid/1"), vec![stored]);
}

#[test]
fn the_stored_folder_and_unknown_folders_find_nothing() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "mangadex", "m-uuid", "Kaiju No. 8");
    assert!(original(&lib, "/data/chapters/mangadex/m-uuid/1").is_empty());
    assert!(original(&lib, "/elsewhere/mangadex/m-uuid/2").is_empty());
    assert!(original(&lib, "/elsewhere/mangadex/other/1").is_empty());
}

#[test]
fn the_source_folder_tells_two_series_with_one_id_apart() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "mangadex", "42", "Dex");
    let dot = follow(&lib, "mangadot", "42", "Dot");
    assert_eq!(original(&lib, "/copy/mangadot/42/1"), vec![dot]);
    assert!(original(&lib, "/copy/42/1").is_empty());
}

#[test]
fn a_copy_under_another_source_folder_is_not_that_series() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "mangadex", "42", "Dex");
    assert!(original(&lib, "/copy/mangadot/42/1").is_empty());
}

#[test]
fn a_symlink_to_a_copy_matches_through_its_resolved_spelling() {
    let lib = open_in_memory().unwrap();
    let stored = follow(&lib, "mangadex", "m-uuid", "Kaiju No. 8");
    let spellings = [Path::new("/tmp/link"), Path::new("/copy/mangadex/m-uuid/1")];
    let found = stored_original(&lib, &spellings, &["mangadex"]).unwrap();
    assert_eq!(found, vec![stored]);
}

#[test]
fn a_manga_id_folder_is_not_a_series_name() {
    let root = tempfile::tempdir().unwrap();
    let id_dir = root.path().join("mangadex/m-uuid/1");
    let named = root.path().join("Kaiju No. 8/1");
    std::fs::create_dir_all(&id_dir).unwrap();
    std::fs::create_dir_all(&named).unwrap();
    let sources = ["mangadex", "local"];
    assert_eq!(series_folder_name(&id_dir, &sources), None);
    assert_eq!(
        series_folder_name(&named, &sources).as_deref(),
        Some("Kaiju No. 8")
    );
}
