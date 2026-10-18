//! `providers options`: key=value assignments checked against a provider's effective options.

use mangas_core::library::Settings;
use mangas_providers::mangadex::MangaDexOptions;
use mangas_providers::options::{display_value, options, reset_options, set_options};
use serde_json::json;

fn set(settings: &Settings, assignments: &[&str]) -> mangas_core::domain::Result<Settings> {
    let owned: Vec<String> = assignments.iter().map(|a| a.to_string()).collect();
    set_options("mangadex", settings, &owned)
}

#[test]
fn shows_defaults_when_nothing_is_stored() {
    let shown = options("mangadex", &Settings::default()).unwrap();
    assert_eq!(shown["data_saver"], json!(false));
    assert_eq!(shown["content_ratings"], json!(["safe", "suggestive"]));
    assert_eq!(display_value(&shown["content_ratings"]), "safe,suggestive");
}

#[test]
fn stores_typed_values_the_provider_reads_back() {
    let next = set(
        &Settings::default(),
        &[
            "data_saver=on",
            "preferred_groups= A , B ",
            "content_ratings=safe",
        ],
    )
    .unwrap();
    let read = MangaDexOptions::from_settings(&next);
    assert!(read.data_saver);
    assert_eq!(read.preferred_groups, ["A", "B"]);
    assert_eq!(read.content_ratings, ["safe"]);
    let cleared = set(&next, &["preferred_groups="]).unwrap();
    assert!(MangaDexOptions::from_settings(&cleared)
        .preferred_groups
        .is_empty());
    assert!(MangaDexOptions::from_settings(&cleared).data_saver);
}

#[test]
fn rejects_unknown_keys_bad_values_and_malformed_assignments() {
    let base = Settings::default();
    let unknown = set(&base, &["datasaver=true"]).unwrap_err().to_string();
    assert!(unknown.contains("data_saver"), "{unknown}");
    assert!(set(&base, &["data_saver=maybe"]).is_err());
    assert!(set(&base, &["data_saver"]).is_err());
    assert!(set_options("nope", &base, &[]).is_err());
}

#[test]
fn values_the_provider_would_ignore_are_rejected() {
    let base = Settings::default();
    let err = set(&base, &["content_ratings=nsfw"])
        .unwrap_err()
        .to_string();
    assert!(err.contains("would read as 'safe,suggestive'"), "{err}");
    let ok = set(&base, &["content_ratings=safe,erotica"]).unwrap();
    let shown = options("mangadex", &ok).unwrap();
    assert_eq!(display_value(&shown["content_ratings"]), "safe,erotica");
}

#[test]
fn providers_without_options_say_so() {
    let base = Settings::default();
    assert!(options("tapas", &base).unwrap().is_empty());
    let err = set_options("tapas", &base, &["x=1".into()]).unwrap_err();
    assert!(err.to_string().contains("tapas has no options"), "{err}");
}

#[test]
fn reset_drops_the_stored_entry() {
    let next = set(&Settings::default(), &["data_saver=true"]).unwrap();
    let reset = reset_options("mangadex", &next).unwrap();
    assert!(!reset.provider_options.contains_key("mangadex"));
}
