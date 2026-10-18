//! Pure mapping from MangaDex wire types to domain types.

use super::api::{ChapterData, Collection, Entity, ErrorBody, Localized, MangaData, Relationship};
use super::dedupe::FeedChapter;
use mangas_core::domain::{Availability, Chapter, ChapterId, Error, Manga, MangaId, MangaStatus};

pub const SOURCE_NAME: &str = "mangadex";
const COVER_BASE: &str = "https://uploads.mangadex.org/covers";

pub(super) fn source_error(message: impl Into<String>) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message: message.into(),
    }
}

pub(super) fn parse<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, Error> {
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
    pub chapters: Vec<FeedChapter>,
    pub next_offset: Option<u32>,
}

/// `GET /manga/{id}/feed` body -> chapters of `language`. Externally hosted ones (`externalUrl`,
/// e.g. MANGA Plus) are kept as [`Availability::External`]; unavailable (removed) and page-less
/// uploads are dropped.
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
        .filter(|c| is_listed(c))
        .map(|c| FeedChapter {
            chapter: chapter_from_data(c, manga_id),
            published_at: c.attributes.publish_at.clone(),
        })
        .collect();
    Ok(FeedPage {
        chapters,
        next_offset: next_offset(col.limit, col.offset, col.total),
    })
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

pub fn cover_url(manga_id: &str, file_name: &str) -> String {
    format!("{COVER_BASE}/{manga_id}/{file_name}.256.jpg")
}

fn next_offset(limit: u32, offset: u32, total: u32) -> Option<u32> {
    let next = offset.saturating_add(limit);
    (limit > 0 && next < total).then_some(next)
}

fn is_listed(c: &ChapterData) -> bool {
    let a = &c.attributes;
    !a.is_unavailable && (external_url(c).is_some() || a.pages != Some(0))
}

fn external_url(c: &ChapterData) -> Option<&str> {
    let url = c.attributes.external_url.as_deref()?.trim();
    (!url.is_empty()).then_some(url)
}

fn availability(c: &ChapterData) -> Availability {
    match external_url(c) {
        Some(url) => Availability::External {
            url: secure_link(url),
        },
        None => Availability::Available,
    }
}

/// The link to open for an externally hosted chapter: a well-formed `https` URL with a host,
/// `http` upgraded to `https` (every site MangaDex links to serves it), anything else no link.
pub fn secure_link(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() == "http" {
        parsed.set_scheme("https").ok()?;
    }
    let has_host = parsed.host_str().is_some_and(|h| !h.is_empty());
    (parsed.scheme() == "https" && has_host).then(|| parsed.into())
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
        authors: names(&m.relationships, "author"),
        artists: names(&m.relationships, "artist"),
        year: attrs.year,
        tags: attrs.tags.iter().filter_map(|t| pick_text(&t.attributes.name)).collect(),
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
        group: group_names(&c.relationships),
        availability: availability(c),
    }
}

/// Names of the `scanlation_group` relationships (needs `includes[]=scanlation_group`).
fn group_names(relationships: &[Relationship]) -> Option<String> {
    let names = names(relationships, "scanlation_group");
    (!names.is_empty()).then(|| names.join(", "))
}

/// Names of the expanded relationships of `kind`, once each (a mangaka is often both author
/// and artist, but never twice in one list).
fn names(relationships: &[Relationship], kind: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for name in relationships
        .iter()
        .filter(|r| r.kind == kind)
        .filter_map(|r| r.attributes.as_ref()?.name.as_deref())
        .filter(|n| !n.is_empty())
    {
        if !found.iter().any(|f| f == name) {
            found.push(name.to_string());
        }
    }
    found
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
