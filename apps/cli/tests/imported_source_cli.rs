//! #259: commands left without `--source` act on the source the library follows the title on, so
//! an imported title is not "not in your library", and an unfollowed one still says so.

use std::path::Path;
use std::process::{Command, Output};

use mangas_core::domain::{Manga, MangaId};
use mangas_core::library;

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

fn stderr(output: &Output) -> String {
    assert!(!output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Imports a one-chapter "Quiet Harbor" through `mangas import`; returns its library id.
fn import(home: &Path) -> String {
    let chapter = home.join("Quiet Harbor/1");
    std::fs::create_dir_all(&chapter).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(chapter.join("001.png")).unwrap();
    let series = home.join("Quiet Harbor");
    let out = stdout(&mangas(home, &["import", series.to_str().unwrap()]));
    let line = out.lines().find(|l| l.starts_with("Library id: ")).unwrap();
    let id = line["Library id: ".len()..].split(' ').next().unwrap();
    assert!(line.ends_with("(source local)"), "{out}");
    id.to_string()
}

#[test]
fn download_of_an_imported_title_says_there_is_nothing_to_download() {
    let home = tempfile::tempdir().unwrap();
    let id = import(home.path());
    let err = stderr(&mangas(home.path(), &["download", &id]));
    assert!(!err.contains("not in your library"), "{err}");
    assert!(err.contains("is an imported title"), "{err}");
    assert!(err.contains("nothing to download"), "{err}");
    let err = stderr(&mangas(
        home.path(),
        &["download", &id, "--source", "local"],
    ));
    assert!(err.contains("already on disk"), "{err}");
}

#[test]
fn a_title_the_library_does_not_follow_is_still_named_as_such() {
    let home = tempfile::tempdir().unwrap();
    import(home.path());
    let err = stderr(&mangas(home.path(), &["download", "no-such-title"]));
    let line = err.lines().last().unwrap();
    assert!(
        line.starts_with("error: no-such-title is not in your library; run "),
        "{line}"
    );
    assert!(line.contains("mangas add no-such-title"), "{line}");
    let err = stderr(&mangas(home.path(), &["remove", "no-such-title"]));
    assert!(
        err.contains("no-such-title is not in your library"),
        "{err}"
    );
}

#[test]
fn the_other_followed_title_commands_resolve_the_imported_source() {
    let home = tempfile::tempdir().unwrap();
    let id = import(home.path());
    let groups = stdout(&mangas(home.path(), &["group", &id]));
    assert!(
        groups.contains(&format!("No groups credited for {id}")),
        "{groups}"
    );
    let chapters = stdout(&mangas(home.path(), &["chapters", &id, "--lang", "en"]));
    assert!(!chapters.contains("not in your library"), "{chapters}");
    let out = home.path().join("books");
    let args = ["export", &id, "--dry-run", "-o", out.to_str().unwrap()];
    let plan = stdout(&mangas(home.path(), &args));
    assert!(plan.contains("Quiet Harbor"), "{plan}");
    let err = stderr(&mangas(home.path(), &["storage", "delete", &id]));
    assert!(!err.contains("not in your library"), "{err}");
    let removed = stdout(&mangas(home.path(), &["remove", &id]));
    assert!(removed.starts_with(&format!("Removed {id}")), "{removed}");
}

#[test]
fn an_id_followed_on_two_sources_needs_source() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("data/mangas")).unwrap();
    let lib = library::open(&home.path().join("data/mangas/mangas.db")).unwrap();
    for source in ["mangadex", "mangadot"] {
        let manga = Manga {
            id: MangaId("42".into()),
            source: source.into(),
            title: "Twin".into(),
            ..Default::default()
        };
        library::upsert_manga(&lib, &manga).unwrap();
    }
    let err = stderr(&mangas(home.path(), &["group", "42"]));
    assert!(
        err.contains("42 is followed on mangadex and mangadot; pick one with --source"),
        "{err}"
    );
    for args in [&["chapters", "42", "--offline"][..], &["info", "42"]] {
        let err = stderr(&mangas(home.path(), args));
        assert!(err.contains("pick one with --source"), "{args:?}: {err}");
    }
    let ok = stdout(&mangas(
        home.path(),
        &["group", "42", "--source", "mangadot"],
    ));
    assert!(ok.contains("No groups credited for 42"), "{ok}");
}
