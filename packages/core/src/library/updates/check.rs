use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use futures::future::join_all;
use futures::stream::{self, StreamExt};
use rusqlite::params;

use super::UpdateReport;
use super::{new_chapters, select, CheckFailure, Known, Target, TitleUpdate, UpdatePolicy};
use crate::domain::{Chapter, Error, MangaId, Result};
use crate::library::{fetch_chapters, get_manga, list_entries, store_chapters, Library};
use crate::sources::Source;

/// A source ready to query, with the chapter languages to sync from it.
pub struct SourceFor {
    pub source: Arc<dyn Source>,
    pub languages: Vec<String>,
}

/// Builds the source for a provider id; an error fails every title of that provider.
pub type Resolve<'a> = dyn Fn(&str) -> Result<SourceFor> + Sync + 'a;

/// Titles refreshed at once per provider. The provider's own limiter paces the requests.
const PER_PROVIDER: usize = 2;

enum Outcome {
    New(TitleUpdate),
    Nothing,
    Failed(CheckFailure),
}

/// Re-syncs the followed titles the policy selects, every provider in parallel and at most
/// [`PER_PROVIDER`] titles of each at a time. A title that fails is reported, not fatal.
pub async fn check(
    lib: &Library,
    resolve: &Resolve<'_>,
    policy: &UpdatePolicy,
) -> Result<UpdateReport> {
    let (targets, skipped) = select(&list_entries(lib)?, policy);
    let mut by_source: BTreeMap<String, Vec<Target>> = BTreeMap::new();
    for target in targets {
        by_source
            .entry(target.manga.source.clone())
            .or_default()
            .push(target);
    }
    let checked = by_source.values().map(Vec::len).sum();
    let runs = by_source
        .into_iter()
        .map(|(name, targets)| check_source(lib, resolve(&name), targets));
    let mut report = UpdateReport {
        checked,
        skipped,
        ..UpdateReport::default()
    };
    for outcome in join_all(runs).await.into_iter().flatten() {
        match outcome {
            Outcome::New(update) => report.updates.push(update),
            Outcome::Failed(failure) => report.failures.push(failure),
            Outcome::Nothing => {}
        }
    }
    let by_title = |t: &str| t.to_lowercase();
    report.updates.sort_by_key(|u| by_title(&u.title));
    report
        .failures
        .sort_by_key(|f| (f.source.clone(), by_title(&f.title)));
    Ok(report)
}

async fn check_source(
    lib: &Library,
    source: Result<SourceFor>,
    targets: Vec<Target>,
) -> Vec<Outcome> {
    let source = match source {
        Ok(source) => source,
        Err(e) => return targets.iter().map(|t| failed(t, &e)).collect(),
    };
    stream::iter(targets)
        .map(|target| check_title(lib, &source, target))
        .buffer_unordered(PER_PROVIDER)
        .collect()
        .await
}

async fn check_title(lib: &Library, source: &SourceFor, target: Target) -> Outcome {
    match refresh(lib, source, &target).await {
        Ok(chapters) if chapters.is_empty() => Outcome::Nothing,
        Ok(chapters) => Outcome::New(TitleUpdate {
            source: target.manga.source,
            manga_id: target.manga.id,
            title: target.manga.title,
            auto_download: target.auto_download,
            chapter_group: target.chapter_group,
            chapters,
        }),
        Err(e) => failed(&target, &e),
    }
}

/// Fetches and stores the title's chapter lists; returns the new ones.
async fn refresh(lib: &Library, source: &SourceFor, target: &Target) -> Result<Vec<Chapter>> {
    let (name, id) = (target.manga.source.as_str(), &target.manga.id);
    let known = known(lib, name, id, target.chapter_group.as_deref())?;
    let lists = fetch_chapters(source.source.as_ref(), id, &source.languages).await?;
    // Unfollowed while the request ran: storing now would bring the chapters back.
    if get_manga(lib, name, id)?.is_none() {
        return Ok(Vec::new());
    }
    let new = new_chapters(&known, &lists);
    store_chapters(lib, name, id, lists)?;
    Ok(new)
}

fn known(lib: &Library, source: &str, id: &MangaId, group: Option<&str>) -> Result<Known> {
    let conn = lib.conn()?;
    let column = |sql: &str| -> Result<HashSet<String>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![source, id.0], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    };
    Ok(Known {
        chapter_ids: column("SELECT id FROM chapter WHERE source = ?1 AND manga_id = ?2")?,
        languages: column(
            "SELECT language FROM manga_language_sync WHERE source = ?1 AND manga_id = ?2",
        )?,
        numbers: numbers(&conn, source, id, group)?,
    })
}

fn numbers(
    conn: &rusqlite::Connection,
    source: &str,
    id: &MangaId,
    group: Option<&str>,
) -> Result<HashSet<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT language, TRIM(number) FROM chapter WHERE source = ?1 AND manga_id = ?2
            AND (?3 IS NULL OR scanlation_group = ?3)",
    )?;
    let rows = stmt.query_map(params![source, id.0, group], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn failed(target: &Target, error: &Error) -> Outcome {
    Outcome::Failed(CheckFailure {
        source: target.manga.source.clone(),
        manga_id: target.manga.id.clone(),
        title: target.manga.title.clone(),
        error: error.to_string(),
        challenge: matches!(error, Error::Challenge { .. }),
    })
}
