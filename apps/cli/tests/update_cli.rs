//! `mangas update` on a fixture library: skip rules, per-title failures (exit 2), unknown
//! providers, JSON report.

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

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn reports_failures_with_exit_2_after_printing_the_rest() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());

    let out = mangas(home.path(), &["update"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let shown = text(&out.stdout);
    assert!(shown.contains("Not checked: GHOST (retired)"), "{shown}");
    assert!(
        shown.contains(
            "1 provider unreachable (checked 1 title, 1 title not checked on retired, skipped 2"
        ),
        "{shown}"
    );
    assert!(
        text(&out.stderr).contains("not checked: GHOST (retired)"),
        "{out:?}"
    );

    let out = mangas(home.path(), &["update", "--json"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["checked"], 1);
    assert_eq!(report["skipped"], 2);
    assert_eq!(report["updates"].as_array().unwrap().len(), 0);
    assert_eq!(report["failures"].as_array().unwrap().len(), 1);
    assert_eq!(report["failures"][0]["manga_id"], "ghost");
    assert_eq!(report["failures"][0]["challenge"], false);
}

#[test]
fn a_run_without_failures_exits_0_and_skips_finished_titles() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());

    let only = stdout(&mangas(home.path(), &["update", "--source", "mangadex"]));
    assert!(
        only.starts_with("Up to date (checked 0 titles, skipped 3"),
        "{only}"
    );
    let json = stdout(&mangas(
        home.path(),
        &["update", "--source", "mangadex", "--json"],
    ));
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(report["failures"], serde_json::json!([]));
}

#[test]
fn an_unknown_source_is_an_error_not_an_empty_check() {
    let home = tempfile::tempdir().unwrap();
    fixture(home.path());

    let out = mangas(home.path(), &["update", "--source", "nosuch"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        text(&out.stderr).contains("unknown provider 'nosuch'"),
        "{out:?}"
    );
    assert!(out.stdout.is_empty(), "{out:?}");
}
