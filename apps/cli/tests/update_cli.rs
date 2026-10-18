//! `mangas update` on a fixture library: skip rules, per-title failures, JSON report.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Manga, MangaId};
use mangas_core::library::{self, set_reading_status, upsert_manga, ReadingStatus};

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

/// Two finished titles on MangaDex and one on a provider this build doesn't have, so the
/// check runs end to end without touching the network.
fn fixture(home: &Path) {
    let dir = home.join("data/mangas");
    std::fs::create_dir_all(&dir).unwrap();
    let lib = library::open(&dir.join("mangas.db")).unwrap();
    for (source, id, status) in [
        ("mangadex", "done", ReadingStatus::Completed),
        ("mangadex", "gone", ReadingStatus::Dropped),
        ("retired", "ghost", ReadingStatus::Reading),
    ] {
        let manga = Manga {
            id: MangaId(id.into()),
            source: source.into(),
            title: id.to_uppercase(),
            ..Default::default()
        };
        upsert_manga(&lib, &manga).unwrap();
        set_reading_status(&lib, source, &manga.id, status).unwrap();
    }
}

#[test]
fn skips_finished_titles_and_reports_failures_without_failing() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());

    let text = stdout(&mangas(home.path(), &["update"]));
    assert!(text.contains("Not checked: GHOST (retired)"), "{text}");
    assert!(text.contains("1 provider unreachable"), "{text}");
    assert!(text.contains("checked 1 title, skipped 2"), "{text}");

    let only = stdout(&mangas(home.path(), &["update", "--source", "mangadex"]));
    assert!(
        only.starts_with("Up to date (checked 0 titles, skipped 3"),
        "{only}"
    );

    let json = stdout(&mangas(home.path(), &["update", "--json"]));
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(report["checked"], 1);
    assert_eq!(report["skipped"], 2);
    assert_eq!(report["updates"].as_array().unwrap().len(), 0);
    assert_eq!(report["failures"][0]["manga_id"], "ghost");
    assert_eq!(report["failures"][0]["challenge"], false);
}
