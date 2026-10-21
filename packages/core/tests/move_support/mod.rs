//! A data-folder move killed for real: the test binary re-runs itself as the mover
//! (`child_moves_until_killed`) and the parent SIGKILLs it once the first file is copied.

#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use mangas_core::domain::{ChapterId, MangaId};
use mangas_core::library::{
    mark_downloaded, open, set_queue_held_until, upsert_chapters, upsert_manga, Library,
};
use mangas_core::storage::{move_data_dir, Folders};

pub fn folders(root: &Path) -> Folders {
    Folders {
        data_dir: root.join("data"),
        database: root.join("app/mangas.db"),
        covers: root.join("app/covers"),
        logs: root.join("app/logs"),
    }
}

const ROOT_VAR: &str = "MANGAS_MOVE_TEST_ROOT";
const TO_VAR: &str = "MANGAS_MOVE_TEST_TO";

pub fn library_with_two_chapters(f: &Folders) -> Library {
    let data = &f.data_dir;
    fs::create_dir_all(f.database.parent().unwrap()).unwrap();
    let lib = open(&f.database).unwrap();
    let manga = mangas_core::domain::Manga {
        id: MangaId("a".into()),
        source: "mangadex".into(),
        title: "A".into(),
        ..Default::default()
    };
    upsert_manga(&lib, &manga).unwrap();
    let rows: Vec<_> = ["1", "2"]
        .iter()
        .map(|n| mangas_core::domain::Chapter {
            id: ChapterId(format!("a-{n}")),
            manga_id: MangaId("a".into()),
            language: "en".into(),
            number: (*n).into(),
            ..Default::default()
        })
        .collect();
    upsert_chapters(&lib, "mangadex", &MangaId("a".into()), "en", &rows).unwrap();
    for n in ["1", "2"] {
        let dir = data.join("chapters/mangadex/a").join(n);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("001.jpg"), vec![7u8; 100]).unwrap();
        mark_downloaded(&lib, "mangadex", &ChapterId(format!("a-{n}")), &dir).unwrap();
    }
    lib
}

#[test]
#[ignore = "run by killed_move"]
fn child_moves_until_killed() {
    let (Ok(root), Ok(to)) = (std::env::var(ROOT_VAR), std::env::var(TO_VAR)) else {
        return;
    };
    let f = folders(Path::new(&root));
    let lib = open(&f.database).unwrap();
    let ready = Path::new(&root).join("ready");
    let _ = move_data_dir(&lib, &f.data_dir, Path::new(&to), &mut |p| {
        if p.copied > 0 {
            fs::write(&ready, b"").unwrap();
            std::thread::sleep(Duration::from_secs(600));
        }
    });
}

/// Moves to `to` in a child process, SIGKILLs it after the first file is copied, then lets
/// the queue hold expire as a dead process's lease does.
pub fn killed_move(lib: &Library, root: &Path, to: &Path) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "move_support::child_moves_until_killed",
            "--exact",
            "--ignored",
        ])
        .env(ROOT_VAR, root)
        .env(TO_VAR, to)
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !root.join("ready").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the move never copied a file"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    set_queue_held_until(lib, 0).unwrap();
}
