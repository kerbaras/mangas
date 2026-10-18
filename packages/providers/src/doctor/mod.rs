//! `providers check`: walks one provider the way a download would (status → search → manga →
//! chapters → pages → first page image) and reports each step with its timing.

mod languages;
mod report;

use std::future::Future;
use std::time::Instant;

use mangas_core::domain::{Chapter, Manga};
use mangas_core::sources::{Header, ProviderStatus, Source};

pub use languages::first_language;
pub use report::{lines, Report, Step, StepReport, STEPS};

use crate::http::PageClient;

/// Search results asked for, and how many of them are tried until one has chapters (popular
/// titles often have none hosted in a given language).
const SEARCH_LIMIT: usize = 10;
const CANDIDATES: usize = 5;

/// Runs the check against the real site. The image step is a one-byte ranged GET of the first
/// page through the downloader's [`PageClient`]: same page headers, pacing, retries and
/// public-hosts-only rule.
pub async fn check(source: &dyn Source, query: &str, language: &str) -> Report {
    let client = PageClient::for_source(source).map_err(|e| e.to_string());
    let client = &client;
    check_with(source, query, language, |url, _headers| async move {
        let client = client.as_ref().map_err(Clone::clone)?;
        client.first_byte(&url).await.map_err(|e| e.to_string())
    })
    .await
}

/// [`check`] with the image fetch given: `fetch(url, headers)` returns the HTTP status.
pub async fn check_with<F, Fut>(
    source: &dyn Source,
    query: &str,
    language: &str,
    fetch: F,
) -> Report
where
    F: Fn(String, Vec<Header>) -> Fut,
    Fut: Future<Output = Result<u16, String>>,
{
    let mut steps = Vec::new();
    walk(source, query, language, &fetch, &mut steps).await;
    Report {
        provider: source.name().to_string(),
        steps,
    }
}

async fn walk<F, Fut>(
    source: &dyn Source,
    query: &str,
    language: &str,
    fetch: &F,
    steps: &mut Vec<StepReport>,
) -> Option<()>
where
    F: Fn(String, Vec<Header>) -> Fut,
    Fut: Future<Output = Result<u16, String>>,
{
    run(steps, Step::Status, async { status(source.status().await) }).await?;
    let found = run(steps, Step::Search, search(source, query)).await?;
    run(steps, Step::Manga, async {
        let manga = source
            .manga(&found[0].id)
            .await
            .map_err(|e| e.to_string())?;
        Ok((format!("{} ({})", manga.title, manga.id), ()))
    })
    .await?;
    let chapter = run(
        steps,
        Step::Chapters,
        first_chapter(source, &found, language),
    )
    .await?;
    let pages = run(steps, Step::Pages, async {
        let pages = source.pages(&chapter.id).await.map_err(|e| e.to_string())?;
        let count = pages.0.len();
        match pages.0.into_iter().next() {
            Some(first) => Ok((format!("{count} in chapter {}", label(&chapter)), first)),
            None => Err(format!("no pages in chapter {}", label(&chapter))),
        }
    })
    .await?;
    run(steps, Step::Image, async {
        let status = fetch(pages.clone(), source.page_headers(&pages)).await?;
        if (200..300).contains(&status) {
            Ok((format!("HTTP {status} {pages}"), ()))
        } else {
            Err(format!("HTTP {status} {pages}"))
        }
    })
    .await
}

/// Times one step and records it; `None` stops the walk.
async fn run<T>(
    steps: &mut Vec<StepReport>,
    step: Step,
    work: impl Future<Output = Result<(String, T), String>>,
) -> Option<T> {
    let start = Instant::now();
    let result = work.await;
    let (outcome, value) = match result {
        Ok((detail, value)) => (Ok(detail), Some(value)),
        Err(reason) => (Err(reason), None),
    };
    steps.push(StepReport {
        step,
        elapsed: start.elapsed(),
        outcome,
    });
    value
}

fn status(status: ProviderStatus) -> Result<(String, ()), String> {
    match status {
        ProviderStatus::Ready => Ok(("ready".to_string(), ())),
        ProviderStatus::NeedsSession => {
            Err("needs a browser session (connect in the app: Settings > Providers)".to_string())
        }
        other => Err(other.to_string()),
    }
}

async fn search(source: &dyn Source, query: &str) -> Result<(String, Vec<Manga>), String> {
    let found = source
        .search(query, SEARCH_LIMIT)
        .await
        .map_err(|e| e.to_string())?;
    match found.len() {
        0 => Err(format!("no results for \"{query}\"")),
        n => Ok((format!("{n} results for \"{query}\""), found)),
    }
}

/// The first downloadable chapter of the first result (among [`CANDIDATES`]) that has one in
/// `language`; locked and externally hosted chapters have no pages to check. The results passed
/// over are named so the title differing from the manga step is explained.
async fn first_chapter(
    source: &dyn Source,
    found: &[Manga],
    language: &str,
) -> Result<(String, Chapter), String> {
    let mut skipped = Vec::new();
    for manga in found.iter().take(CANDIDATES) {
        let chapters = source
            .chapters(&manga.id, language)
            .await
            .map_err(|e| e.to_string())?;
        let count = chapters.len();
        if let Some(first) = chapters.into_iter().find(|c| c.is_available()) {
            let used = format!("{count} ({language}) in \"{}\"", manga.title);
            return Ok((with_skipped(used, &skipped, language), first));
        }
        skipped.push(format!("\"{}\"", manga.title));
    }
    Err(format!(
        "no {language} chapters in the first {} results: {}",
        skipped.len(),
        skipped.join(", ")
    ))
}

fn with_skipped(detail: String, skipped: &[String], language: &str) -> String {
    match skipped {
        [] => detail,
        titles => format!(
            "{detail}; skipped {}: no {language} chapters",
            titles.join(", ")
        ),
    }
}

fn label(chapter: &Chapter) -> String {
    match chapter.number.as_str() {
        "" => chapter.id.to_string(),
        number => format!("{number} ({})", chapter.id),
    }
}
