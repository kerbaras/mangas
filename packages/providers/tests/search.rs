use mangas_core::library::Settings;
use mangas_providers::{providers, search_enabled, searchable, ProviderStatus};

fn ids(settings: &Settings, only: &[String]) -> Vec<&'static str> {
    searchable(settings, only).iter().map(|p| p.id).collect()
}

#[test]
fn every_provider_is_searched_until_switched_off() {
    let all: Vec<_> = providers().iter().map(|p| p.id).collect();
    assert_eq!(ids(&Settings::default(), &[]), all);

    let settings = Settings {
        providers_enabled: [("mangadot".to_string(), false)].into(),
        ..Settings::default()
    };
    assert!(!ids(&settings, &[]).contains(&"mangadot"));
    assert_eq!(ids(&settings, &[]).len(), all.len() - 1);
}

#[test]
fn only_narrows_the_enabled_providers() {
    let settings = Settings {
        providers_enabled: [("mangadot".to_string(), false)].into(),
        ..Settings::default()
    };
    let only = [
        "mangadot".to_string(),
        "mangadex".to_string(),
        "nope".to_string(),
    ];
    assert_eq!(ids(&settings, &only), ["mangadex"]);
}

#[tokio::test]
async fn a_provider_without_its_session_does_not_fail_the_search() {
    // An empty query sends no request: this checks grouping and order without the network.
    let groups = search_enabled(&Settings::default(), " ", 5, &[]).await;
    let names: Vec<_> = groups.iter().map(|g| g.provider.as_str()).collect();
    let all: Vec<_> = providers().iter().map(|p| p.id).collect();
    assert_eq!(names, all);
    assert!(groups
        .iter()
        .all(|g| g.status == ProviderStatus::Ready && g.items.is_empty()));
}
