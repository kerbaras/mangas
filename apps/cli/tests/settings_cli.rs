//! `mangas settings languages` and `mangas providers options` persist to the shared library.

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
fn preferred_languages_round_trip() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(
        stdout(&mangas(home.path(), &["settings", "languages"])),
        "en\n"
    );
    let saved = mangas(home.path(), &["settings", "languages", "ES-LA, en"]);
    assert_eq!(stdout(&saved), "es-la,en\n");
    assert_eq!(
        stdout(&mangas(home.path(), &["settings", "languages"])),
        "es-la,en\n"
    );
    assert!(!mangas(home.path(), &["settings", "languages", " , "])
        .status
        .success());
    let bad = mangas(home.path(), &["settings", "languages", "es,english"]);
    assert!(!bad.status.success());
    let err = String::from_utf8_lossy(&bad.stderr);
    assert!(err.contains("'english' is not a language code"), "{err}");
    assert_eq!(
        stdout(&mangas(home.path(), &["settings", "languages"])),
        "es-la,en\n"
    );
}

#[test]
fn provider_options_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let set = &[
        "providers",
        "options",
        "mangadex",
        "--set",
        "data_saver=yes",
    ];
    assert!(stdout(&mangas(home.path(), set)).contains("data_saver = true"));
    let shown = stdout(&mangas(home.path(), &["providers", "options", "mangadex"]));
    assert!(shown.contains("data_saver = true"), "{shown}");
    assert!(
        shown.contains("content_ratings = safe,suggestive"),
        "{shown}"
    );
    let reset = &["providers", "options", "mangadex", "--reset"];
    assert!(stdout(&mangas(home.path(), reset)).contains("data_saver = false"));
    let bad = &["providers", "options", "mangadex", "--set", "nope=1"];
    assert!(!mangas(home.path(), bad).status.success());
}
