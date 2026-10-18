//! Moving a data folder reached through a symlink keeps the link and its (now empty) target, so
//! the old path never dangles; only the copied folders go.
#![cfg(unix)]

use std::fs;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    get_settings, list_chapters, mark_downloaded, open_in_memory, upsert_chapters, upsert_manga,
};
use mangas_core::storage::move_data_dir;

#[test]
fn moving_a_linked_data_folder_leaves_no_dangling_link() {
    let tmp = tempfile::tempdir().unwrap();
    let real = tmp.path().join("real-data");
    let link = tmp.path().join("data");
    fs::create_dir_all(&real).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let lib = open_in_memory().unwrap();
    let manga = Manga {
        id: MangaId("a".into()),
        source: "mangadex".into(),
        title: "A".into(),
        ..Default::default()
    };
    upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("a-1".into()),
        manga_id: MangaId("a".into()),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    upsert_chapters(&lib, "mangadex", &manga.id, "en", &[chapter]).unwrap();
    let dir = link.join("chapters/mangadex/a/1");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("001.jpg"), [7u8; 10]).unwrap();
    mark_downloaded(&lib, "mangadex", &ChapterId("a-1".into()), &dir).unwrap();

    let target = tmp.path().join("new");
    move_data_dir(&lib, &link, &target, &mut |_| {}).unwrap();

    assert!(
        fs::symlink_metadata(&link).unwrap().is_symlink(),
        "link kept"
    );
    assert!(link.is_dir(), "the link still points at a folder");
    assert!(!real.join("chapters").exists(), "the copied folders go");
    let target = fs::canonicalize(&target).unwrap();
    let settings = get_settings(&lib).unwrap();
    assert_eq!(settings.data_dir.as_deref(), Some(target.to_str().unwrap()));
    let chapters = list_chapters(&lib, "mangadex", &manga.id, None).unwrap();
    let path = chapters[0].local_path.clone().unwrap();
    assert!(path.starts_with(target.to_str().unwrap()), "{path}");
}
