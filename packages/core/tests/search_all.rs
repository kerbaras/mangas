use mangas_core::library::Network;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, MangaStatus, Result};
use mangas_core::sources::search::{group, in_order, timed_out, total};
use mangas_core::sources::{search_all, PageUrls, ProviderStatus, SearchGroup, Source};

fn manga(source: &str, id: &str) -> Manga {
    Manga {
        id: MangaId(id.into()),
        source: source.into(),
        title: format!("Title {id}"),
        description: String::new(),
        cover_url: None,
        status: MangaStatus::Ongoing,
        ..Default::default()
    }
}

enum Behaviour {
    Answer(Vec<&'static str>),
    Challenge,
    Fail,
    Hang,
}

struct Fake {
    name: &'static str,
    behaviour: Behaviour,
}

#[async_trait]
impl Source for Fake {
    fn name(&self) -> &'static str {
        self.name
    }
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        assert_eq!(query, "berserk", "the query reaches the source trimmed");
        match &self.behaviour {
            Behaviour::Answer(ids) => Ok(ids
                .iter()
                .take(limit)
                .map(|id| manga(self.name, id))
                .collect()),
            Behaviour::Challenge => Err(Error::Challenge {
                name: self.name.into(),
                site: "https://example.com".into(),
            }),
            Behaviour::Fail => Err(Error::Source {
                name: self.name.into(),
                message: "HTTP 503".into(),
            }),
            Behaviour::Hang => {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(Vec::new())
            }
        }
    }
    async fn manga(&self, _: &MangaId) -> Result<Manga> {
        unimplemented!()
    }
    async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>> {
        unimplemented!()
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls> {
        unimplemented!()
    }
}

fn fake(name: &'static str, behaviour: Behaviour) -> Arc<dyn Source> {
    Arc::new(Fake { name, behaviour })
}

#[tokio::test]
async fn failing_providers_get_their_own_group_and_never_sink_the_search() {
    let sources = [
        fake("ok", Behaviour::Answer(vec!["1", "2", "1"])),
        fake("gated", Behaviour::Challenge),
        fake("down", Behaviour::Fail),
        fake("slow", Behaviour::Hang),
    ];
    let started = std::time::Instant::now();
    let groups = search_all(
        &sources,
        "  berserk ",
        10,
        Duration::from_millis(200),
        &Network::Direct,
    )
    .await;
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the timeout bounds the search"
    );

    let names: Vec<_> = groups.iter().map(|g| g.provider.as_str()).collect();
    assert_eq!(names, ["ok", "gated", "down", "slow"]);
    assert_eq!(groups[0].status, ProviderStatus::Ready);
    let ids: Vec<_> = groups[0].items.iter().map(|m| m.id.0.as_str()).collect();
    assert_eq!(ids, ["1", "2"], "repeated ids are dropped");
    assert_eq!(groups[1].status, ProviderStatus::NeedsSession);
    assert!(
        matches!(&groups[2].status, ProviderStatus::Unreachable { reason } if reason.contains("HTTP 503"))
    );
    assert_eq!(
        groups[3],
        timed_out("slow", Duration::from_millis(200), &Network::Direct)
    );
    assert_eq!(total(&groups), 2);
    assert!(groups[0].is_ok() && !groups[1].is_ok());
}

#[tokio::test]
async fn sources_run_concurrently_under_one_deadline() {
    let sources = [fake("a", Behaviour::Hang), fake("b", Behaviour::Hang)];
    let started = std::time::Instant::now();
    let groups = search_all(
        &sources,
        "berserk",
        10,
        Duration::from_millis(300),
        &Network::Direct,
    )
    .await;
    assert!(
        started.elapsed() < Duration::from_millis(550),
        "{:?}",
        started.elapsed()
    );
    assert!(groups.iter().all(|g| !g.is_ok()));
}

#[tokio::test]
async fn an_empty_query_sends_nothing() {
    let sources = [fake("down", Behaviour::Fail)];
    let groups = search_all(
        &sources,
        "   ",
        10,
        Duration::from_secs(1),
        &Network::Direct,
    )
    .await;
    assert_eq!(groups, [group("down", Ok(Vec::new()))]);
}

#[test]
fn groups_follow_the_registry_order() {
    let g = |p: &str| group(p, Ok(Vec::new()));
    let ordered = in_order(vec![g("x"), g("b"), g("a")], &["a", "b"]);
    let names: Vec<_> = ordered.iter().map(|g| g.provider.as_str()).collect();
    assert_eq!(names, ["a", "b", "x"]);
}

#[test]
fn groups_serialise_with_the_status_tag() {
    let g: SearchGroup = group("down", Err(Error::Other("boom".into())));
    let json = serde_json::to_value(&g).unwrap();
    assert_eq!(json["provider"], "down");
    assert_eq!(json["status"]["state"], "unreachable");
    assert_eq!(json["status"]["reason"], "boom");
    assert_eq!(json["items"], serde_json::json!([]));
}
