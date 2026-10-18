//! Live smoke tests against the real sites. Ignored by default and skipped unless
//! `MANGAS_LIVE=1`: `MANGAS_LIVE=1 cargo test -p mangas-providers --test live -- --ignored`.
//! MangaDot is behind a Cloudflare verification, so only MangaDex, WeebCentral, Tapas and
//! Webtoons run here.
//! Comick's search is gated the same way, so its tests walk the open endpoints and check the
//! missing-session report.

mod contract;

use std::io::Write;

use contract::{headers, manifest, walk};
use mangas_core::domain::{Error, MangaId};
use mangas_core::library::{ProviderSession, Settings};
use mangas_providers::{doctor, info, source, ProviderStatus};

/// Whether live tests run. When they don't, says so on the real stderr (libtest captures
/// `eprintln!`), so `cargo test -- --ignored` printing `ok` is not mistaken for a pass.
fn live(test: &str) -> bool {
    let on = std::env::var("MANGAS_LIVE").is_ok_and(|v| v == "1");
    if !on {
        let _ = writeln!(
            std::io::stderr(),
            "skipped {test}: set MANGAS_LIVE=1 to run it against the real site"
        );
    }
    on
}

/// The fixture contract, against the real site (same query and language as the fixtures).
async fn live_contract(id: &str) {
    let info = info(id).unwrap();
    let manifest = manifest::load(id).unwrap();
    let source = source(id, &Settings::default()).unwrap();
    assert_eq!(source.status().await, ProviderStatus::Ready, "{id}: status");
    let walked = walk::assert_walk(&*source, &info, &manifest.query, &manifest.language).await;
    let need_referer = manifest.pages_need_referer;
    headers::assert_page_headers(&*source, &info, &walked.pages, need_referer, None);
}

/// `mangas providers check <id>`: the full walk including the first page image.
async fn live_doctor(id: &str) {
    let manifest = manifest::load(id).unwrap();
    let source = source(id, &Settings::default()).unwrap();
    let report = doctor::check(&*source, &manifest.query, &manifest.language).await;
    let table = doctor::lines(&report).join("\n");
    println!("{table}");
    assert!(report.passed(), "{id}:\n{table}");
}

#[tokio::test]
#[ignore = "hits api.mangadex.org; run with MANGAS_LIVE=1"]
async fn mangadex_live_contract() {
    if live("mangadex_live_contract") {
        live_contract("mangadex").await;
    }
}

#[tokio::test]
#[ignore = "hits api.mangadex.org; run with MANGAS_LIVE=1"]
async fn mangadex_live_check() {
    if live("mangadex_live_check") {
        live_doctor("mangadex").await;
    }
}

#[tokio::test]
#[ignore = "hits weebcentral.com; run with MANGAS_LIVE=1"]
async fn weebcentral_live_contract() {
    if live("weebcentral_live_contract") {
        live_contract("weebcentral").await;
    }
}

#[tokio::test]
#[ignore = "hits weebcentral.com; run with MANGAS_LIVE=1"]
async fn weebcentral_live_check() {
    if live("weebcentral_live_check") {
        live_doctor("weebcentral").await;
    }
}

#[tokio::test]
#[ignore = "hits tapas.io; run with MANGAS_LIVE=1"]
async fn tapas_live_contract() {
    if live("tapas_live_contract") {
        live_contract("tapas").await;
    }
}

#[tokio::test]
#[ignore = "hits tapas.io; run with MANGAS_LIVE=1"]
async fn tapas_live_check() {
    if live("tapas_live_check") {
        live_doctor("tapas").await;
    }
}

/// A Comick title with chapters in English, for the endpoints that need no session.
const COMICK_SLUG: &str = "00-solo-leveling";

#[tokio::test]
#[ignore = "hits comick.live; run with MANGAS_LIVE=1"]
async fn comick_live_open_endpoints() {
    if !live("comick_live_open_endpoints") {
        return;
    }
    let info = info("comick").unwrap();
    let source = source("comick", &Settings::default()).unwrap();
    assert_eq!(source.status().await, ProviderStatus::NeedsSession);
    let id = MangaId(COMICK_SLUG.into());
    let manga = source.manga(&id).await.unwrap();
    assert_eq!(manga.id, id);
    let chapters = source.chapters(&id, "en").await.unwrap();
    assert!(chapters.len() > 100, "{} chapters", chapters.len());
    assert!(chapters.iter().all(|c| c.language == "en"));
    let pages = source.pages(&chapters[0].id).await.unwrap();
    assert!(!pages.0.is_empty(), "{}: no pages", chapters[0].id);
    headers::assert_page_headers(&*source, &info, &pages.0, true, None);
}

#[tokio::test]
#[ignore = "hits comick.live; run with MANGAS_LIVE=1"]
async fn comick_live_check_reports_needs_session() {
    if !live("comick_live_check_reports_needs_session") {
        return;
    }
    let manifest = manifest::load("comick").unwrap();
    let source = source("comick", &Settings::default()).unwrap();
    let report = doctor::check(&*source, &manifest.query, &manifest.language).await;
    let table = doctor::lines(&report).join("\n");
    println!("{table}");
    assert!(!report.passed());
    assert!(table.contains("needs a browser session"), "{table}");
}

#[tokio::test]
#[ignore = "hits comick.live; run with MANGAS_LIVE=1"]
async fn comick_live_stale_session_search_is_a_challenge() {
    if !live("comick_live_stale_session_search_is_a_challenge") {
        return;
    }
    let mut settings = Settings::default();
    let stale = ProviderSession {
        cookies: "cf_clearance=stale".into(),
        user_agent: "Mozilla/5.0 (X11; Linux x86_64) Gecko/20100101 Firefox/140.0".into(),
        saved_at: None,
    };
    settings.provider_sessions.insert("comick".into(), stale);
    let source = source("comick", &settings).unwrap();
    let err = source.search("solo leveling", 5).await.unwrap_err();
    assert!(matches!(err, Error::Challenge { .. }), "{err:?}");
    assert_eq!(source.status().await, ProviderStatus::NeedsSession);
}

#[tokio::test]
#[ignore = "hits webtoons.com; run with MANGAS_LIVE=1"]
async fn webtoons_live_contract() {
    if live("webtoons_live_contract") {
        live_contract("webtoons").await;
    }
}

#[tokio::test]
#[ignore = "hits webtoons.com; run with MANGAS_LIVE=1"]
async fn webtoons_live_check() {
    if live("webtoons_live_check") {
        live_doctor("webtoons").await;
    }
}
