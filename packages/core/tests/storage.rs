use std::fs;
use std::path::{Path, PathBuf};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    get_manga, get_settings, list_chapters, mark_downloaded, open_in_memory, set_settings,
    upsert_chapters, upsert_manga, Library, Settings,
};
use mangas_core::storage::{
    clear_cache, delete_downloads, move_data_dir, usage, Cache, Folders, MoveProgress,
};

fn manga(source: &str, id: &str, title: &str) -> Manga {
    Manga {
        id: MangaId(id.into()),
        source: source.into(),
        title: title.into(),
        ..Default::default()
    }
}

fn chapter(manga: &str, id: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        manga_id: MangaId(manga.into()),
        title: None,
        language: "en".into(),
        volume: None,
        number: id.into(),
        local_path: None,
        group: None,
        availability: Default::default(),
    }
}

fn folders(root: &Path) -> Folders {
    Folders {
        data_dir: root.join("data"),
        database: root.join("app/mangas.db"),
        covers: root.join("app/covers"),
        logs: root.join("app/logs"),
    }
}

fn write(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![7u8; bytes]).unwrap();
}

/// Follows `source/id` with downloaded chapters of `pages` bytes each under `data`.
fn downloaded(lib: &Library, data: &Path, source: &str, id: &str, chapters: &[(&str, usize)]) {
    upsert_manga(lib, &manga(source, id, &format!("Title {id}"))).unwrap();
    let key = |c: &str| format!("{id}-{c}");
    let rows: Vec<_> = chapters.iter().map(|(c, _)| chapter(id, &key(c))).collect();
    upsert_chapters(lib, source, &MangaId(id.into()), "en", &rows).unwrap();
    for (c, bytes) in chapters {
        let dir = data.join("chapters").join(source).join(id).join(c);
        write(&dir.join("001.jpg"), *bytes);
        mark_downloaded(lib, source, &ChapterId(key(c)), &dir).unwrap();
    }
}

#[test]
fn usage_sums_chapters_per_title_largest_first() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    downloaded(&lib, &f.data_dir, "mangadex", "a", &[("1", 100)]);
    downloaded(
        &lib,
        &f.data_dir,
        "mangadex",
        "b",
        &[("1", 300), ("2", 200)],
    );
    write(&f.covers.join("x.jpg"), 40);
    write(&f.models().join("m.onnx"), 1000);
    let u = usage(&lib, &f).unwrap();
    assert_eq!((u.chapters, u.covers, u.models), (600, 40, 1000));
    let titles: Vec<_> = u
        .titles
        .iter()
        .map(|t| (t.manga_id.as_str(), t.chapters, t.bytes))
        .collect();
    assert_eq!(titles, vec![("b", 2, 500), ("a", 1, 100)]);
}

#[test]
fn deleting_downloads_keeps_the_title_and_its_chapter_list() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    downloaded(&lib, &f.data_dir, "mangadex", "a", &[("1", 100), ("2", 50)]);
    let deleted = delete_downloads(&lib, &f.chapters(), "mangadex", "a").unwrap();
    assert_eq!((deleted.chapters, deleted.bytes), (2, 150));
    assert!(!f.chapters().join("mangadex/a").exists());
    assert!(get_manga(&lib, "mangadex", &MangaId("a".into()))
        .unwrap()
        .is_some());
    let chapters = list_chapters(&lib, "mangadex", &MangaId("a".into()), None).unwrap();
    assert_eq!(chapters.len(), 2);
    assert!(chapters.iter().all(|c| c.local_path.is_none()));
}

#[test]
fn deleting_leaves_folders_outside_the_download_layout_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &manga("mangadex", "a", "A")).unwrap();
    upsert_chapters(
        &lib,
        "mangadex",
        &MangaId("a".into()),
        "en",
        &[chapter("a", "1")],
    )
    .unwrap();
    let odd = tmp.path().join("my-folder");
    write(&odd.join("001.jpg"), 10);
    mark_downloaded(&lib, "mangadex", &ChapterId("1".into()), &odd).unwrap();
    let deleted = delete_downloads(&lib, &f.chapters(), "mangadex", "a").unwrap();
    assert_eq!(deleted.chapters, 0);
    assert_eq!(deleted.kept, vec![odd.to_string_lossy().into_owned()]);
    assert!(odd.join("001.jpg").exists());
}

#[test]
fn deleting_skips_look_alike_folders_outside_the_data_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    downloaded(
        &lib,
        &tmp.path().join("elsewhere"),
        "mangadex",
        "a",
        &[("1", 10)],
    );
    let deleted = delete_downloads(&lib, &f.chapters(), "mangadex", "a").unwrap();
    assert_eq!((deleted.chapters, deleted.kept.len()), (0, 1));
    assert!(tmp.path().join("elsewhere/chapters/mangadex/a").exists());
}

#[test]
fn deleting_also_removes_chapter_archives() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    downloaded(&lib, &f.data_dir, "mangadex", "a", &[("1", 100)]);
    let dir = list_chapters(&lib, "mangadex", &MangaId("a".into()), None).unwrap()[0]
        .local_path
        .clone()
        .unwrap();
    let cbz = mangas_core::download::cbz_path(Path::new(&dir));
    write(&cbz, 40);
    let deleted = delete_downloads(&lib, &f.chapters(), "mangadex", "a").unwrap();
    assert_eq!(deleted.bytes, 140);
    assert!(!cbz.exists());
}

#[test]
fn imported_titles_cant_have_their_downloads_deleted() {
    let lib = open_in_memory().unwrap();
    assert!(delete_downloads(&lib, Path::new("/data/chapters"), "local", "x").is_err());
}

#[test]
fn clearing_the_cover_cache_empties_it_and_keeps_the_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    write(&f.covers.join("a.jpg"), 10);
    write(&f.covers.join("b.webp"), 5);
    let cleared = clear_cache(&f, Cache::Covers).unwrap();
    assert_eq!((cleared.files, cleared.bytes), (2, 15));
    assert!(f.covers.is_dir());
    assert_eq!(fs::read_dir(&f.covers).unwrap().count(), 0);
    let none = clear_cache(&f, Cache::Models).unwrap();
    assert_eq!(none.files, 0);
}

#[test]
fn moving_copies_verifies_switches_and_removes_the_old_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    set_settings(
        &lib,
        &Settings {
            data_dir: Some(f.data_dir.to_string_lossy().into()),
            default_device: Some("kindle-scribe".into()),
            ..Default::default()
        },
    )
    .unwrap();
    downloaded(&lib, &f.data_dir, "mangadex", "a", &[("1", 100), ("2", 50)]);
    write(&f.models().join("m.onnx"), 25);
    let target = tmp.path().join("big-disk/mangas");
    let mut seen = Vec::new();
    let moved = move_data_dir(&lib, &f.data_dir, &target, &mut |p: MoveProgress| {
        seen.push(p)
    })
    .unwrap();
    let target = fs::canonicalize(&target).unwrap();
    assert_eq!((moved.files, moved.bytes), (3, 175));
    assert!(moved.leftovers.is_empty());
    assert_eq!(seen.last().map(|p| (p.copied, p.total)), Some((175, 175)));
    assert!(!f.chapters().exists() && !f.models().exists());
    assert!(target.join("models/m.onnx").exists());
    let settings = get_settings(&lib).unwrap();
    assert_eq!(settings.data_dir.as_deref(), Some(target.to_str().unwrap()));
    assert_eq!(settings.default_device.as_deref(), Some("kindle-scribe"));
    let chapters = list_chapters(&lib, "mangadex", &MangaId("a".into()), None).unwrap();
    for c in chapters {
        let path = PathBuf::from(c.local_path.unwrap());
        assert!(
            path.starts_with(target.join("chapters/mangadex/a")),
            "{path:?}"
        );
        assert!(path.join("001.jpg").exists());
    }
}

#[test]
fn moving_into_a_folder_with_downloads_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    downloaded(&lib, &f.data_dir, "mangadex", "a", &[("1", 100)]);
    let target = tmp.path().join("other");
    write(&target.join("chapters/x/y/1/001.jpg"), 1);
    let err = move_data_dir(&lib, &f.data_dir, &target, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("choose an empty folder"), "{err}");
    assert!(f.chapters().join("mangadex/a/1/001.jpg").exists());
    assert_eq!(get_settings(&lib).unwrap().data_dir, None);
    assert!(move_data_dir(
        &lib,
        &f.data_dir,
        &f.data_dir.join("chapters/sub"),
        &mut |_| {}
    )
    .is_err());
}

#[cfg(unix)]
#[test]
fn moving_copies_the_contents_of_linked_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    downloaded(&lib, &f.data_dir, "mangadex", "a", &[("1", 100)]);
    let page = tmp.path().join("outside.jpg");
    write(&page, 30);
    std::os::unix::fs::symlink(&page, f.chapters().join("mangadex/a/1/002.jpg")).unwrap();
    let target = tmp.path().join("new");
    let moved = move_data_dir(&lib, &f.data_dir, &target, &mut |_| {}).unwrap();
    assert_eq!((moved.files, moved.bytes), (2, 130));
    let copied = fs::canonicalize(&target)
        .unwrap()
        .join("chapters/mangadex/a/1/002.jpg");
    assert_eq!(fs::metadata(copied).unwrap().len(), 30);
}

#[cfg(unix)]
#[test]
fn deleting_keeps_chapters_recorded_as_links() {
    let tmp = tempfile::tempdir().unwrap();
    let f = folders(tmp.path());
    let lib = open_in_memory().unwrap();
    downloaded(&lib, &f.data_dir, "mangadex", "a", &[("1", 10)]);
    let manga = f.chapters().join("mangadex/a");
    fs::rename(manga.join("1"), manga.join("real")).unwrap();
    std::os::unix::fs::symlink(manga.join("real"), manga.join("1")).unwrap();
    write(&manga.join("real.cbz"), 5);
    let deleted = delete_downloads(&lib, &f.chapters(), "mangadex", "a").unwrap();
    assert_eq!((deleted.chapters, deleted.kept.len()), (0, 1));
    assert!(manga.join("real.cbz").exists() && manga.join("real/001.jpg").exists());
}
