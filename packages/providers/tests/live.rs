//! Live smoke tests against the real sites. Ignored by default and skipped unless
//! `MANGAS_LIVE=1`: `MANGAS_LIVE=1 cargo test -p mangas-providers --test live -- --ignored`.
//! MangaDot is behind a Cloudflare verification, so only MangaDex runs here.

mod contract;

use contract::{headers, manifest, walk};
use mangas_core::library::Settings;
use mangas_providers::{doctor, info, source, ProviderStatus};

fn live() -> bool {
    std::env::var("MANGAS_LIVE").is_ok_and(|v| v == "1")
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
    if live() {
        live_contract("mangadex").await;
    }
}

#[tokio::test]
#[ignore = "hits api.mangadex.org; run with MANGAS_LIVE=1"]
async fn mangadex_live_check() {
    if live() {
        live_doctor("mangadex").await;
    }
}
