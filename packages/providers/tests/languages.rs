//! Matching saved chapter languages to a provider's editions, regional codes falling back to
//! their base language.

use mangas_providers::{language_for, languages_for, offered_language, resolve_languages};

fn strings(codes: &[&str]) -> Vec<String> {
    codes.iter().map(|c| c.to_string()).collect()
}

#[test]
fn exact_codes_win_over_the_base_language() {
    let offered = ["en", "es", "es-la", "pt-br"];
    assert_eq!(offered_language("es-la", &offered), Some("es-la"));
    assert_eq!(offered_language("ES-LA", &offered), Some("es-la"));
    assert_eq!(offered_language("es", &offered), Some("es"));
    assert_eq!(offered_language("es-la", &["en", "es"]), Some("es"));
    assert_eq!(offered_language("pt-pt", &["pt"]), Some("pt"));
    assert_eq!(
        offered_language("pt", &offered),
        None,
        "no region is guessed"
    );
    assert_eq!(offered_language("ja", &offered), None);
}

#[test]
fn resolution_keeps_preference_order_without_duplicates() {
    let webtoons = ["en", "es", "fr"];
    assert_eq!(
        resolve_languages(&strings(&["es-la", "en"]), &webtoons),
        ["es", "en"]
    );
    assert_eq!(
        resolve_languages(&strings(&["es-la", "es", "ja"]), &webtoons),
        ["es"]
    );
    assert!(resolve_languages(&strings(&["ja"]), &webtoons).is_empty());
}

#[test]
fn registered_providers_resolve_through_their_info() {
    assert_eq!(languages_for("webtoons", &strings(&["es-la"])), ["es"]);
    assert_eq!(languages_for("mangadex", &strings(&["es-la"])), ["es-la"]);
    assert_eq!(
        languages_for("tapas", &strings(&["en-gb", "es"])),
        ["en", "es"]
    );
    assert_eq!(
        languages_for("webtoons", &strings(&["es-la", "es"])),
        ["es"]
    );
    // Not in the provider's list, or no such provider: ask as given (MangaDex carries Arabic).
    assert_eq!(
        languages_for("mangadex", &strings(&["ar", "en"])),
        ["ar", "en"]
    );
    assert_eq!(languages_for("tapas", &strings(&["ja"])), ["ja"]);
    assert_eq!(languages_for("local", &strings(&["es-la"])), ["es-la"]);
    assert_eq!(language_for("webtoons", "es-la"), "es");
    assert_eq!(language_for("weebcentral", "fr"), "fr");
}
