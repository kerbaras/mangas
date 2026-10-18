use mangas_core::library::{self, merge_edit, Network, ProxyMode, Settings, Theme};

fn stored_json(path: &std::path::Path) -> serde_json::Value {
    let raw: String = rusqlite::Connection::open(path)
        .unwrap()
        .query_row("SELECT value FROM setting WHERE key = 'app'", [], |r| {
            r.get(0)
        })
        .unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn only_changed_keys_are_stored() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mangas.db");
    let lib = library::open(&path).unwrap();
    let edited = Settings {
        theme: Some(Theme::Dark),
        download_concurrency: Some(2),
        ..Settings::default()
    };
    library::edit_settings(&lib, Some(&Settings::default()), &edited).unwrap();
    assert_eq!(
        stored_json(&path),
        serde_json::json!({ "theme": "dark", "download_concurrency": 2 })
    );

    // Reset to defaults drops the keys again; the other process's change stays.
    let other = Settings {
        proxy_mode: Some(ProxyMode::None),
        ..edited.clone()
    };
    library::set_settings(&lib, &other).unwrap();
    library::edit_settings(&lib, Some(&edited), &Settings::default()).unwrap();
    assert_eq!(
        stored_json(&path),
        serde_json::json!({ "proxy_mode": "none" })
    );
}

#[test]
fn concurrency_defaults_to_four_and_stays_in_range() {
    let with = |n| Settings {
        download_concurrency: n,
        ..Settings::default()
    };
    assert_eq!(with(None).concurrency(), library::DEFAULT_CONCURRENCY);
    assert_eq!(with(Some(0)).concurrency(), 1);
    assert_eq!(with(Some(6)).concurrency(), 6);
    assert_eq!(with(Some(99)).concurrency(), 8);
}

#[test]
fn network_follows_the_proxy_mode() {
    let with = |mode, url: Option<&str>| {
        Settings {
            proxy_mode: mode,
            proxy_url: url.map(String::from),
            ..Settings::default()
        }
        .network()
    };
    assert_eq!(with(None, Some("http://p:1")), Network::System);
    assert_eq!(with(Some(ProxyMode::None), None), Network::Direct);
    assert_eq!(
        with(Some(ProxyMode::Custom), Some(" 127.0.0.1:8888 ")),
        Network::Proxy("http://127.0.0.1:8888".into())
    );
    // A custom mode without a usable URL keeps today's behaviour instead of failing.
    assert_eq!(with(Some(ProxyMode::Custom), None), Network::System);
    assert_eq!(
        with(Some(ProxyMode::Custom), Some("ftp://p")),
        Network::System
    );
}

#[test]
fn proxy_urls_need_http_and_a_host() {
    assert_eq!(
        library::proxy_url("https://user:pw@proxy.lan:3128").unwrap(),
        "https://user:pw@proxy.lan:3128"
    );
    assert!(library::proxy_url("socks5://proxy:1080").is_err());
    assert!(library::proxy_url("http://").is_err());
    assert!(library::proxy_url("").is_err());
}

#[test]
fn merge_keeps_a_theme_changed_elsewhere() {
    let base = Settings::default();
    let current = Settings {
        theme: Some(Theme::Light),
        ..Settings::default()
    };
    let edited = Settings {
        download_concurrency: Some(8),
        ..Settings::default()
    };
    let merged = merge_edit(&base, &current, &edited).unwrap();
    assert_eq!(merged.theme, Some(Theme::Light));
    assert_eq!(merged.download_concurrency, Some(8));
}

#[test]
fn command_line_networks_parse_and_store() {
    assert_eq!(Network::parse("System").unwrap(), Network::System);
    assert_eq!(Network::parse("none").unwrap(), Network::Direct);
    let proxy = Network::parse("localhost:8888").unwrap();
    assert_eq!(proxy, Network::Proxy("http://localhost:8888".into()));
    assert!(Network::parse("socks5://x:1").is_err());

    let custom = proxy.store(&Settings::default());
    assert_eq!(custom.network(), proxy);
    let off = Network::Direct.store(&custom);
    assert_eq!(off.proxy_mode, Some(ProxyMode::None));
    assert_eq!(off.proxy_url, custom.proxy_url, "the URL is kept for later");
    assert_eq!(Network::System.store(&off).proxy_mode, None);
}
