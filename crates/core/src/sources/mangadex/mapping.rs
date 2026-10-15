//! Pure mapping from MangaDex wire types to domain types.

use std::collections::HashSet;

use super::api::{AtHome, ChapterData, Collection, Entity, ErrorBody, Localized, MangaData};
use crate::domain::{sort_by_number, Chapter, ChapterId, Error, Manga, MangaId, MangaStatus};
use crate::sources::PageUrls;

pub const SOURCE_NAME: &str = "mangadex";
const COVER_BASE: &str = "https://uploads.mangadex.org/covers";

fn source_error(message: impl Into<String>) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message: message.into(),
    }
}

fn parse<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, Error> {
    serde_json::from_str(body).map_err(|e| source_error(format!("unexpected response: {e}")))
}

/// `GET /manga?title=..` body -> search results.
pub fn parse_search_response(body: &str) -> Result<Vec<Manga>, Error> {
    let col: Collection<MangaData> = parse(body)?;
    Ok(col.data.iter().map(manga_from_data).collect())
}

/// `GET /manga/{id}` body -> manga.
pub fn parse_manga_response(body: &str) -> Result<Manga, Error> {
    let entity: Entity<MangaData> = parse(body)?;
    Ok(manga_from_data(&entity.data))
}

/// One page of `GET /manga/{id}/feed`: the downloadable chapters plus the paging totals.
pub struct FeedPage {
    pub chapters: Vec<Chapter>,
    pub next_offset: Option<u32>,
}

/// `GET /manga/{id}/feed` body -> chapters of `language` that MangaDex actually hosts.
/// Externally hosted (`externalUrl`) and unavailable chapters are dropped.
pub fn parse_feed_response(
    body: &str,
    manga_id: &MangaId,
    language: &str,
) -> Result<FeedPage, Error> {
    let col: Collection<ChapterData> = parse(body)?;
    let chapters = col
        .data
        .iter()
        .filter(|c| c.attributes.translated_language == language)
        .filter(|c| is_hosted(c))
        .map(|c| chapter_from_data(c, manga_id))
        .collect();
    Ok(FeedPage {
        chapters,
        next_offset: next_offset(col.limit, col.offset, col.total),
    })
}

/// `GET /at-home/server/{chapter}` body -> full-quality page URLs.
pub fn parse_at_home_response(body: &str) -> Result<PageUrls, Error> {
    let at_home: AtHome = parse(body)?;
    let base = at_home.base_url.trim_end_matches('/');
    let hash = at_home.chapter.hash;
    Ok(PageUrls(
        at_home
            .chapter
            .data
            .iter()
            .map(|file| format!("{base}/data/{hash}/{file}"))
            .collect(),
    ))
}

/// Error response body (`{"result":"error","errors":[..]}`) -> [`Error::Source`].
pub fn parse_error_response(status: u16, body: &str) -> Error {
    let detail = serde_json::from_str::<ErrorBody>(body)
        .ok()
        .and_then(|b| b.errors.into_iter().next())
        .map(|e| e.detail.unwrap_or(e.title))
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| format!("HTTP {status}"));
    source_error(detail)
}

/// Keeps one chapter per number (the first in feed order) and sorts by number ascending.
pub fn dedupe_and_sort(chapters: Vec<Chapter>) -> Vec<Chapter> {
    let mut seen = HashSet::new();
    let unique = chapters
        .into_iter()
        .filter(|c| seen.insert(c.number.clone()))
        .collect();
    sort_by_number(unique)
}

pub fn cover_url(manga_id: &str, file_name: &str) -> String {
    format!("{COVER_BASE}/{manga_id}/{file_name}.512.jpg")
}

fn next_offset(limit: u32, offset: u32, total: u32) -> Option<u32> {
    let next = offset.saturating_add(limit);
    (limit > 0 && next < total).then_some(next)
}

fn is_hosted(c: &ChapterData) -> bool {
    !c.attributes.is_unavailable && c.attributes.external_url.is_none()
}

fn manga_from_data(m: &MangaData) -> Manga {
    let attrs = &m.attributes;
    let title = attrs
        .title
        .get("en")
        .cloned()
        .or_else(|| attrs.alt_titles.iter().find_map(|t| t.get("en").cloned()))
        .filter(|t| !t.is_empty())
        .or_else(|| pick_text(&attrs.title))
        .unwrap_or_else(|| m.id.clone());
    let cover_url = m
        .relationships
        .iter()
        .filter(|r| r.kind == "cover_art")
        .find_map(|r| r.attributes.as_ref()?.file_name.as_deref())
        .map(|file| cover_url(&m.id, file));
    Manga {
        id: MangaId(m.id.clone()),
        source: SOURCE_NAME.to_string(),
        title,
        description: pick_text(&attrs.description).unwrap_or_default(),
        cover_url,
        status: status_from_str(attrs.status.as_deref()),
    }
}

fn chapter_from_data(c: &ChapterData, manga_id: &MangaId) -> Chapter {
    let attrs = &c.attributes;
    Chapter {
        id: ChapterId(c.id.clone()),
        manga_id: manga_id.clone(),
        title: attrs.title.clone().filter(|t| !t.is_empty()),
        language: attrs.translated_language.clone(),
        volume: attrs.volume.clone().filter(|v| !v.is_empty()),
        number: attrs.chapter.clone().unwrap_or_default(),
        local_path: None,
    }
}

/// English first, then a romanised title, then whatever the source has.
fn pick_text(text: &Localized) -> Option<String> {
    ["en", "ja-ro", "ko-ro", "zh-ro"]
        .iter()
        .find_map(|lang| text.get(*lang))
        .or_else(|| text.values().next())
        .cloned()
        .filter(|t| !t.is_empty())
}

fn status_from_str(s: Option<&str>) -> MangaStatus {
    match s {
        Some("ongoing") => MangaStatus::Ongoing,
        Some("completed") => MangaStatus::Completed,
        Some("hiatus") => MangaStatus::Hiatus,
        Some("cancelled") => MangaStatus::Cancelled,
        _ => MangaStatus::Unknown,
    }
}
