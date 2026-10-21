use std::collections::BTreeSet;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use mangas_core::library::{ProviderSession, Settings};
use mangas_providers::http::{answered_gate, probe_status, session_gate};
use mangas_providers::{
    info, providers, source, source_at, status, Capability, ProviderStatus, Session,
};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn every_provider_builds_under_its_own_id() {
    let settings = Settings::default();
    let infos = providers();
    let ids: BTreeSet<_> = infos.iter().map(|p| p.id).collect();
    assert_eq!(ids.len(), infos.len(), "duplicate provider ids");
    for p in &infos {
        assert_eq!(source(p.id, &settings).unwrap().name(), p.id);
        assert_eq!(info(p.id), Some(*p));
        assert!(p.site.starts_with("https://"), "{}", p.site);
        assert!(!p.languages.is_empty());
        assert!(p.capabilities.contains(&Capability::Search));
    }
}

#[test]
fn unknown_ids_list_the_known_ones() {
    let err = source("nope", &Settings::default()).err().unwrap();
    assert!(
        err.to_string().contains("mangadex, mangadot, weebcentral"),
        "{err}"
    );
    assert_eq!(info("nope"), None);
}

#[test]
fn info_serialises_for_the_apps() {
    let json = serde_json::to_value(info("comick").unwrap()).unwrap();
    assert_eq!(json["needs_browser_session"], true);
    let optional = serde_json::to_value(info("mangadot").unwrap()).unwrap();
    assert_eq!(optional["needs_browser_session"], false);
    assert_eq!(json["capabilities"][0], "search");
    let status = serde_json::to_value(ProviderStatus::Unreachable { reason: "x".into() });
    assert_eq!(
        status.unwrap(),
        serde_json::json!({"state": "unreachable", "reason": "x"})
    );
}

/// Comick's probe against a mock base: offline and deterministic (#312).
async fn comick_status(base: &str) -> ProviderStatus {
    let settings = Settings::default();
    source_at("comick", &settings, base).unwrap().status().await
}

async fn answering(status: u16) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(status).set_body_string("{}"))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn a_missing_session_needs_one_once_the_host_answers() {
    for code in [200, 403] {
        let server = answering(code).await;
        assert_eq!(
            comick_status(&server.uri()).await,
            ProviderStatus::NeedsSession
        );
    }
    let failing = answering(503).await;
    assert!(comick_status(&failing.uri()).await.is_unreachable());
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    assert!(comick_status(&dead).await.is_unreachable());
    let unknown = status("nope", &Settings::default()).await;
    assert!(matches!(unknown, ProviderStatus::Unreachable { .. }));
}

#[test]
fn stored_sessions_reach_the_source() {
    let mut settings = Settings::default();
    settings.provider_sessions.insert(
        "mangadot".into(),
        ProviderSession {
            cookies: "cf_clearance=abc".into(),
            user_agent: "UA".into(),
            saved_at: None,
        },
    );
    let source = source("mangadot", &settings).unwrap();
    let headers = source.page_headers("https://mangadot.net/x.webp");
    assert!(headers.contains(&("cookie".into(), "cf_clearance=abc".into())));
}

#[test]
fn sessions_expire_after_their_ttl() {
    let now = Utc.with_ymd_and_hms(2026, 10, 17, 12, 0, 0).unwrap();
    let day = Duration::from_secs(86_400);
    let session = |saved_at: Option<&str>| Session {
        cookies: "c=1".into(),
        user_agent: String::new(),
        saved_at: saved_at.map(String::from),
    };
    assert!(session(Some("2026-10-15T12:00:00Z")).is_expired(day, now));
    assert!(!session(Some("2026-10-17T11:00:00+00:00")).is_expired(day, now));
    assert!(!session(Some("garbage")).is_expired(day, now));
    assert!(!session(None).is_expired(day, now));

    let old = session(Some("2026-01-01T00:00:00Z"));
    assert_eq!(
        session_gate(true, Some(&old), Some(day), now),
        Some(ProviderStatus::NeedsSession)
    );
    assert_eq!(session_gate(true, Some(&old), None, now), None);
    assert_eq!(
        session_gate(true, None, None, now),
        Some(ProviderStatus::NeedsSession)
    );
    assert_eq!(session_gate(false, None, None, now), None);
}

#[test]
fn probe_results_map_to_status() {
    use mangas_core::domain::Error;
    let ok: mangas_core::domain::Result<()> = Ok(());
    assert_eq!(probe_status(&ok), ProviderStatus::Ready);
    let challenge: mangas_core::domain::Result<()> = Err(Error::Challenge {
        name: "x".into(),
        site: "https://x".into(),
    });
    assert_eq!(probe_status(&challenge), ProviderStatus::NeedsSession);
}

#[test]
fn a_session_gate_holds_only_once_the_host_answers() {
    use mangas_core::domain::Error;
    let gate = Some(ProviderStatus::NeedsSession);
    for status in [200, 403] {
        assert_eq!(answered_gate(gate.clone(), Ok(status)), gate);
    }
    let wall = Error::Challenge {
        name: "x".into(),
        site: "https://x".into(),
    };
    assert_eq!(answered_gate(gate.clone(), Err(&wall)), gate);
    assert_eq!(answered_gate(gate.clone(), Ok(500)), None);
    assert_eq!(answered_gate(None, Ok(200)), None);
    for gate in [gate, None] {
        let status = answered_gate(gate, Err(&Error::NoRootCertificates));
        assert!(status.is_some_and(|s| s.is_unreachable()));
    }
}
