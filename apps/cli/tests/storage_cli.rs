//! `mangas storage` and `mangas backup` against a temporary home.

use std::path::Path;
use std::process::{Command, Output};

fn mangas(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn usage_lists_the_folders_and_clear_empties_the_cover_cache() {
    let home = tempfile::tempdir().unwrap();
    let covers = home.path().join("data/mangas/covers");
    std::fs::create_dir_all(&covers).unwrap();
    std::fs::write(covers.join("a.jpg"), [0u8; 1500]).unwrap();
    let usage = stdout(&mangas(home.path(), &["storage"]));
    assert!(usage.contains("Cover cache"), "{usage}");
    assert!(usage.contains("1.50 KB"), "{usage}");
    let cleared = stdout(&mangas(home.path(), &["storage", "clear", "covers"]));
    assert!(
        cleared.contains("Cleared the cover cache: 1.50 KB freed"),
        "{cleared}"
    );
    assert!(covers.is_dir() && std::fs::read_dir(&covers).unwrap().next().is_none());
}

#[test]
fn backup_round_trips_through_a_haruneko_bookmark_list() {
    let home = tempfile::tempdir().unwrap();
    let bookmarks = home.path().join("haruneko.bookmarks");
    std::fs::write(
        &bookmarks,
        r#"[{"Created":1,"Updated":1,"Title":"One","Media":{"ProviderID":"mangadex","EntryID":"uuid-1"},"Info":{"ProviderID":null,"EntryID":null}}]"#,
    )
    .unwrap();
    let imported = stdout(&mangas(
        home.path(),
        &["backup", "import", bookmarks.to_str().unwrap()],
    ));
    assert!(imported.contains("Followed 1 new titles"), "{imported}");
    assert!(stdout(&mangas(home.path(), &["list"])).contains("One"));

    let file = home.path().join("backup.json");
    let exported = stdout(&mangas(
        home.path(),
        &["backup", "export", file.to_str().unwrap()],
    ));
    assert!(exported.contains("Exported 1 titles"), "{exported}");
    let json = std::fs::read_to_string(&file).unwrap();
    assert!(
        json.contains("\"format\": \"mangas-backup\"") && json.contains("uuid-1"),
        "{json}"
    );
}

#[test]
fn moving_the_data_folder_switches_where_downloads_go() {
    let home = tempfile::tempdir().unwrap();
    let target = home.path().join("big-disk");
    let moved = stdout(&mangas(
        home.path(),
        &["storage", "move", target.to_str().unwrap()],
    ));
    assert!(moved.contains("Moved 0 files"), "{moved}");
    let usage = stdout(&mangas(home.path(), &["storage", "usage"]));
    assert!(usage.contains("big-disk"), "{usage}");
}
