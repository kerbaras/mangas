//! `mangas kindle` defaults: format, and the title looked up in an in-memory library.

use super::*;
use crate::commands::kindle_library::*;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{self, Library};

#[test]
fn kindle_writes_epub_unless_the_flags_say_otherwise() {
    assert_eq!(legacy_format(None, None, None), Format::Epub);
    assert_eq!(
        legacy_format(None, Some(Path::new("b.pdf")), None),
        Format::Pdf
    );
    assert_eq!(legacy_format(Some(Format::Cbz), None, None), Format::Cbz);
    let series = Some(Format::Cbz);
    assert_eq!(legacy_format(None, None, series), Format::Cbz);
    assert_eq!(
        legacy_format(None, Some(Path::new("b.pdf")), series),
        Format::Pdf
    );
}

/// A library following "Kaiju No. 8" with chapter `c1` downloaded to `<root>/m-uuid/c1`.
fn downloaded(root: &Path) -> (Library, PathBuf) {
    let lib = library::open_in_memory().unwrap();
    let id = MangaId("m-uuid".into());
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
        number: "1".into(),
        ..Default::default()
    };
    library::upsert_chapters(&lib, "mangadex", &id, "en", std::slice::from_ref(&chapter)).unwrap();
    let dir = root.join("m-uuid/c1");
    std::fs::create_dir_all(&dir).unwrap();
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
    (lib, dir)
}

fn title(lib: &Library, chapters: &[PathBuf]) -> String {
    default_title(first_series(lib, chapters).unwrap().as_deref(), chapters)
}

#[test]
fn a_downloaded_chapter_is_titled_from_the_library() {
    let root = tempfile::tempdir().unwrap();
    let (lib, dir) = downloaded(&root.path().canonicalize().unwrap());
    let slashed = PathBuf::from(format!("{}/", dir.display()));
    assert_eq!(title(&lib, &[slashed]), "Kaiju No. 8");
    let relative = Path::new("data/m-uuid/c1");
    library::mark_downloaded(&lib, "mangadex", &ChapterId("c1".into()), relative).unwrap();
    assert_eq!(title(&lib, &[relative.into()]), "Kaiju No. 8");
}

#[test]
fn a_mixed_export_is_titled_after_its_first_chapter() {
    let root = tempfile::tempdir().unwrap();
    let (lib, dir) = downloaded(&root.path().canonicalize().unwrap());
    let loose = root.path().join("loose/c9");
    std::fs::create_dir_all(&loose).unwrap();
    assert_eq!(title(&lib, &[dir.clone(), loose.clone()]), "Kaiju No. 8");
    assert_eq!(labels_in(&lib, &[dir, loose]).unwrap().series, None);
}

#[test]
fn a_folder_the_library_does_not_know_keeps_its_parent_name() {
    let root = tempfile::tempdir().unwrap();
    let (lib, _) = downloaded(&root.path().canonicalize().unwrap());
    let other = root.path().join("loose/c9");
    std::fs::create_dir_all(&other).unwrap();
    assert_eq!(title(&lib, std::slice::from_ref(&other)), "loose");
    assert_eq!(default_title(None, &[other]), "loose");
}

#[test]
fn the_series_preset_applies_only_to_folders_of_that_series() {
    let root = tempfile::tempdir().unwrap();
    let (lib, dir) = downloaded(&root.path().canonicalize().unwrap());
    let preset = ExportPreset {
        page_height: Some(0.7),
        ..Default::default()
    };
    library::set_export_preset(&lib, "mangadex", "m-uuid", &preset).unwrap();
    let slashed = PathBuf::from(format!("{}/", dir.display()));
    assert_eq!(series_preset(&lib, &[slashed]).unwrap(), Some(preset));
    let loose = root.path().join("loose/c9");
    std::fs::create_dir_all(&loose).unwrap();
    assert_eq!(series_preset(&lib, &[dir, loose]).unwrap(), None);
}
