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
    assert!(imported.contains("Followed 1 new title ("), "{imported}");
    assert!(stdout(&mangas(home.path(), &["list"])).contains("One"));

    let file = home.path().join("backup.json");
    let exported = stdout(&mangas(
        home.path(),
        &["backup", "export", file.to_str().unwrap()],
    ));
    assert!(exported.contains("Exported 1 title to"), "{exported}");
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

/// A refused move leaves no folder behind for its target, nor for the parents it would create.
#[test]
fn a_refused_move_creates_no_target_folder() {
    let home = tempfile::tempdir().unwrap();
    let reading = mangas_core::storage::hold_for_export(|| {
        Ok::<_, mangas_core::Error>(home.path().join("data/mangas"))
    })
    .unwrap();
    let to = home.path().join("new/deeper/elsewhere");
    let out = mangas(home.path(), &["storage", "move", to.to_str().unwrap()]);
    assert!(!out.status.success(), "{out:?}");
    assert!(!home.path().join("new").exists(), "{out:?}");
    drop(reading);

    let existing = home.path().join("existing");
    std::fs::create_dir(&existing).unwrap();
    let reading = mangas_core::storage::hold_for_export(|| {
        Ok::<_, mangas_core::Error>(home.path().join("data/mangas"))
    })
    .unwrap();
    let out = mangas(
        home.path(),
        &["storage", "move", existing.to_str().unwrap()],
    );
    assert!(!out.status.success(), "{out:?}");
    assert!(existing.is_dir(), "a folder that was there stays");
    drop(reading);

    let out = mangas(home.path(), &["storage", "move", to.to_str().unwrap()]);
    assert!(out.status.success(), "{out:?}");
    assert!(to.is_dir());
}

#[test]
fn backup_import_keeps_preferences_when_the_library_has_titles_unless_asked() {
    let (from, to) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let titles = |home: &Path, id: &str| {
        let list = home.join("list.bookmarks");
        std::fs::write(
            &list,
            format!(r#"[{{"Created":1,"Updated":1,"Title":"{id}","Media":{{"ProviderID":"mangadex","EntryID":"{id}"}},"Info":{{"ProviderID":null,"EntryID":null}}}}]"#),
        )
        .unwrap();
        stdout(&mangas(home, &["backup", "import", list.to_str().unwrap()]));
    };
    titles(from.path(), "theirs");
    stdout(&mangas(from.path(), &["settings", "languages", "es"]));
    let file = from.path().join("backup.json");
    stdout(&mangas(
        from.path(),
        &["backup", "export", file.to_str().unwrap()],
    ));
    titles(to.path(), "mine");
    stdout(&mangas(to.path(), &["settings", "languages", "fr"]));
    let file = file.to_str().unwrap();
    let languages = || stdout(&mangas(to.path(), &["settings", "languages"]));

    let kept = stdout(&mangas(to.path(), &["backup", "import", file]));
    assert!(kept.contains("Kept your preferences"), "{kept}");
    assert!(languages().contains("fr"), "{}", languages());
    let flagged = stdout(&mangas(
        to.path(),
        &["backup", "import", file, "--keep-preferences"],
    ));
    assert!(flagged.contains("Kept your preferences"), "{flagged}");
    let taken = stdout(&mangas(
        to.path(),
        &["backup", "import", file, "--backup-preferences"],
    ));
    assert!(taken.contains("Restored preferences"), "{taken}");
    assert!(languages().contains("es"), "{}", languages());
    let both = mangas(
        to.path(),
        &[
            "backup",
            "import",
            file,
            "--keep-preferences",
            "--backup-preferences",
        ],
    );
    assert!(!both.status.success());
}
