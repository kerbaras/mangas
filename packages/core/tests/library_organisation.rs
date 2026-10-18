use std::path::Path;

use mangas_core::domain::{Availability, Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    create_category, delete_category, list_categories, list_chapters, list_entries,
    mark_downloaded, mark_exported, open_in_memory, remove_manga, rename_category,
    set_manga_categories, set_reading_status, upsert_chapters, upsert_manga, ChapterCounts,
    Library, ReadingStatus,
};

fn follow(lib: &Library, id: &str) -> MangaId {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: id.to_uppercase(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    manga.id
}

fn chapter(manga: &MangaId, id: &str, number: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        manga_id: manga.clone(),
        language: "en".into(),
        number: number.into(),
        ..Default::default()
    }
}

#[test]
fn categories_are_unique_ignoring_case_and_renamable() {
    let lib = open_in_memory().unwrap();
    let reading = create_category(&lib, " To  read ").unwrap();
    assert_eq!(reading.name, "To read");
    let err = create_category(&lib, "to READ").unwrap_err().to_string();
    assert!(err.contains("already exists"), "{err}");
    let fav = create_category(&lib, "Favourites").unwrap();
    assert_eq!(
        list_categories(&lib).unwrap(),
        [reading.clone(), fav.clone()]
    );

    assert_eq!(
        rename_category(&lib, fav.id, "Faves").unwrap().name,
        "Faves"
    );
    assert!(rename_category(&lib, fav.id, "to read").is_err());
    assert!(rename_category(&lib, 999, "Other").is_err());
    delete_category(&lib, reading.id).unwrap();
    assert!(delete_category(&lib, reading.id).is_err());
    assert_eq!(list_categories(&lib).unwrap().len(), 1);
}

#[test]
fn titles_sit_on_many_categories_and_links_cascade() {
    let lib = open_in_memory().unwrap();
    let a = follow(&lib, "a");
    let b = follow(&lib, "b");
    let x = create_category(&lib, "X").unwrap();
    let y = create_category(&lib, "Y").unwrap();
    set_manga_categories(&lib, "mangadex", &a, &[y.id, x.id, x.id]).unwrap();
    set_manga_categories(&lib, "mangadex", &b, &[x.id]).unwrap();
    assert!(set_manga_categories(&lib, "mangadex", &b, &[999]).is_err());
    assert!(set_manga_categories(&lib, "mangadex", &MangaId("zz".into()), &[x.id]).is_err());

    let cats = |lib: &Library, id: &str| {
        list_entries(lib)
            .unwrap()
            .into_iter()
            .find(|e| e.manga.id.0 == id)
            .map(|e| e.categories)
    };
    assert_eq!(cats(&lib, "a"), Some(vec![x.id, y.id]));
    assert_eq!(
        cats(&lib, "b"),
        Some(vec![x.id]),
        "a failed set keeps the old one"
    );

    delete_category(&lib, x.id).unwrap();
    assert_eq!(cats(&lib, "a"), Some(vec![y.id]));
    remove_manga(&lib, "mangadex", &a).unwrap();
    set_manga_categories(&lib, "mangadex", &b, &[]).unwrap();
    assert_eq!(cats(&lib, "b"), Some(vec![]));
}

#[test]
fn reading_status_defaults_to_reading_and_needs_a_followed_title() {
    let lib = open_in_memory().unwrap();
    let a = follow(&lib, "a");
    assert_eq!(
        list_entries(&lib).unwrap()[0].reading_status,
        ReadingStatus::Reading
    );
    set_reading_status(&lib, "mangadex", &a, ReadingStatus::OnHold).unwrap();
    // Re-saving the source record (a refresh) keeps the user's status.
    follow(&lib, "a");
    assert_eq!(
        list_entries(&lib).unwrap()[0].reading_status,
        ReadingStatus::OnHold
    );
    let ghost = MangaId("ghost".into());
    assert!(set_reading_status(&lib, "mangadex", &ghost, ReadingStatus::Dropped).is_err());
}

#[test]
fn exports_mark_chapters_and_drive_the_unread_count() {
    let lib = open_in_memory().unwrap();
    let a = follow(&lib, "a");
    let mut locked = chapter(&a, "c3", "3");
    locked.availability = Availability::Locked;
    let list = [chapter(&a, "c1", "1"), chapter(&a, "c2", "2"), locked];
    upsert_chapters(&lib, "mangadex", &a, "en", &list).unwrap();
    for (id, dir) in [("c1", "/data/a/1"), ("c2", "/data/a/2")] {
        mark_downloaded(&lib, "mangadex", &ChapterId(id.into()), Path::new(dir)).unwrap();
    }

    let marked = mark_exported(&lib, &[Path::new("/data/a/1/"), Path::new("/elsewhere")]).unwrap();
    assert_eq!(marked, 1);
    let chapters = list_chapters(&lib, "mangadex", &a, None).unwrap();
    let exported_at = chapters[0].exported_at.as_deref().unwrap();
    assert!(
        exported_at.ends_with('Z') && exported_at.contains('T'),
        "{exported_at}"
    );
    assert!(chapters[1].exported_at.is_none());

    // A re-sync refreshes metadata without forgetting the export.
    upsert_chapters(&lib, "mangadex", &a, "en", &list).unwrap();
    assert!(list_chapters(&lib, "mangadex", &a, None).unwrap()[0].is_exported());
    assert_eq!(
        list_entries(&lib).unwrap()[0].chapters,
        ChapterCounts {
            total: 3,
            downloaded: 2,
            exported: 1,
            unread: 1,
        }
    );
}
