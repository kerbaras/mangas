//! A library using every piece of state the migrations up to 0008 added.

use std::path::Path;

use mangas_core::domain::{Availability, Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    create_category, delete_category, mark_downloaded, mark_exported, set_auto_download,
    set_export_preset, set_manga_categories, set_reading_status, set_settings, upsert_chapters,
    upsert_manga, Library, ReadingStatus, Settings,
};

pub fn follow(lib: &Library, source: &str, id: &str) -> MangaId {
    let manga = Manga {
        id: MangaId(id.into()),
        source: source.into(),
        title: id.to_uppercase(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    manga.id
}

pub fn chapter(manga: &MangaId, id: &str, language: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        manga_id: manga.clone(),
        language: language.into(),
        number: id.trim_start_matches(char::is_alphabetic).into(),
        ..Default::default()
    }
}

pub fn fill(lib: &Library, downloads: &Path) {
    let a = follow(lib, "mangadex", "a");
    let b = follow(lib, "mangadex", "b");
    let t = follow(lib, "tapas", "t");
    let fav = create_category(lib, "Favourites").unwrap();
    let gone = create_category(lib, "Gone").unwrap();
    let later = create_category(lib, "Later").unwrap();
    delete_category(lib, gone.id).unwrap();
    set_manga_categories(lib, "mangadex", &a, &[fav.id, later.id]).unwrap();
    set_manga_categories(lib, "tapas", &t, &[later.id]).unwrap();
    set_reading_status(lib, "mangadex", &b, ReadingStatus::OnHold).unwrap();
    set_reading_status(lib, "tapas", &t, ReadingStatus::Dropped).unwrap();
    set_auto_download(lib, "mangadex", &a, true).unwrap();
    let preset = mangas_core::export::ExportPreset {
        page_height: Some(0.8),
        trim_margins: Some(true),
        ..Default::default()
    };
    set_export_preset(lib, "mangadex", &a.0, &preset).unwrap();

    let mut locked = chapter(&a, "a2", "en");
    locked.availability = Availability::External {
        url: Some("https://mangaplus.example/2".into()),
    };
    locked.group = Some("Group A, Group B".into());
    locked.volume = Some("1".into());
    locked.title = Some("Second".into());
    upsert_chapters(
        lib,
        "mangadex",
        &a,
        "en",
        &[chapter(&a, "a1", "en"), locked],
    )
    .unwrap();
    upsert_chapters(lib, "mangadex", &a, "es", &[chapter(&a, "a1es", "es")]).unwrap();
    // A later sync stamps the new chapter's first_seen_at.
    let newer = [chapter(&a, "a1", "en"), chapter(&a, "a3", "en")];
    upsert_chapters(lib, "mangadex", &a, "en", &newer).unwrap();
    upsert_chapters(lib, "tapas", &t, "en", &[chapter(&t, "t1", "en")]).unwrap();
    let dir = downloads.join("a1");
    std::fs::create_dir_all(&dir).unwrap();
    mark_downloaded(lib, "mangadex", &ChapterId("a1".into()), &dir).unwrap();
    assert_eq!(mark_exported(lib, &[dir.as_path()]).unwrap(), 1);
    set_settings(
        lib,
        &Settings {
            default_device: Some("kindle-scribe".into()),
            update_check_hours: Some(12),
            ..Default::default()
        },
    )
    .unwrap();
}
