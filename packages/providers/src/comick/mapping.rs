//! Pure mapping from comick.live responses to domain types.

use mangas_core::domain::{
    sort_by_number, Chapter, ChapterId, Error, Manga, MangaId, MangaStatus, Result,
};
use mangas_core::sources::PageUrls;
use serde_json::Value;

use super::api::{ChapterEntry, ChapterList, ComicData, ReaderData, SearchResponse};
use super::page_data::{plain_text, script_json};

pub const SOURCE_NAME: &str = "comick";
pub const SITE: &str = "https://comick.live";

fn source_error(message: impl Into<String>) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message: message.into(),
    }
}

fn json<T: serde::de::DeserializeOwned>(body: &str) -> Result<T> {
    serde_json::from_str(body).map_err(|e| source_error(format!("unexpected response: {e}")))
}

/// Comick's numeric status: 1 ongoing, 2 completed, 3 cancelled, 4 hiatus.
pub fn parse_status(raw: Option<i64>) -> MangaStatus {
    match raw {
        Some(1) => MangaStatus::Ongoing,
        Some(2) => MangaStatus::Completed,
        Some(3) => MangaStatus::Cancelled,
        Some(4) => MangaStatus::Hiatus,
        _ => MangaStatus::Unknown,
    }
}

/// Search results; a manga's id is its slug.
pub fn parse_search(body: &str) -> Result<Vec<Manga>> {
    let response: SearchResponse = json(body)?;
    Ok(response
        .data
        .into_iter()
        .map(|item| Manga {
            id: MangaId(item.slug),
            source: SOURCE_NAME.to_string(),
            title: item.title.trim().to_string(),
            description: String::new(),
            cover_url: item.default_thumbnail.filter(|u| !u.is_empty()),
            status: MangaStatus::Unknown,
            ..Default::default()
        })
        .collect())
}

/// The `/comic/{slug}` page: its `#comic-data` JSON.
pub fn parse_comic_page(id: &MangaId, html: &str) -> Result<Manga> {
    let data =
        script_json(html, "comic-data").ok_or_else(|| Error::NotFound(format!("manga {id}")))?;
    let comic: ComicData = json(data)?;
    Ok(Manga {
        direction: None,
        id: MangaId(comic.slug),
        source: SOURCE_NAME.to_string(),
        title: comic.title.trim().to_string(),
        description: comic.desc.as_deref().map(plain_text).unwrap_or_default(),
        cover_url: comic.default_thumbnail.filter(|u| !u.is_empty()),
        status: parse_status(comic.status),
        authors: names(comic.authors.into_iter().map(|p| p.name)),
        artists: names(comic.artists.into_iter().map(|p| p.name)),
        year: comic.year.filter(|y| *y > 0),
        tags: names(
            comic
                .md_comic_md_genres
                .into_iter()
                .map(|g| g.md_genres.name),
        ),
    })
}

/// Trimmed, non-empty names in order.
fn names(raw: impl Iterator<Item = String>) -> Vec<String> {
    raw.map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect()
}

/// One page of the chapter list: the chapters in `language` (unsorted) and the last page
/// number.
pub fn parse_chapter_page(
    body: &str,
    manga: &MangaId,
    language: &str,
) -> Result<(Vec<Chapter>, u32)> {
    let list: ChapterList = json(body)?;
    let chapters = list
        .data
        .iter()
        .filter(|e| e.lang == language)
        .map(|e| to_chapter(e, manga))
        .collect();
    Ok((chapters, list.pagination.last_page))
}

/// All pages of a chapter list, sorted by number.
pub fn sorted(pages: Vec<Vec<Chapter>>) -> Vec<Chapter> {
    sort_by_number(pages.into_iter().flatten().collect())
}

fn to_chapter(entry: &ChapterEntry, manga: &MangaId) -> Chapter {
    let number = text(entry.chap.as_ref());
    let groups = entry.group_name.as_deref().unwrap_or_default();
    Chapter {
        id: chapter_id(manga, &entry.hid, &number, &entry.lang),
        manga_id: manga.clone(),
        title: entry.title.clone().filter(|t| !t.trim().is_empty()),
        language: entry.lang.clone(),
        volume: Some(text(entry.vol.as_ref())).filter(|v| !v.is_empty()),
        number,
        local_path: None,
        exported_at: None,
        group: Some(groups.join(", ")).filter(|g| !g.trim().is_empty()),
        availability: Default::default(),
    }
}

/// Chapter ids are `{hid}-chapter-{chap}-{lang}@{slug}`: the reader page needs the full
/// suffix (404 without it) plus the slug, and the id leads with the release-unique hid so
/// same-numbered releases get distinct download directories.
pub fn chapter_id(manga: &MangaId, hid: &str, number: &str, lang: &str) -> ChapterId {
    ChapterId(format!("{hid}-chapter-{number}-{lang}@{manga}"))
}

/// Path of the reader page of `chapter`, from the id produced by [`chapter_id`].
pub fn chapter_path(chapter: &ChapterId) -> Result<String> {
    match chapter.0.rsplit_once('@') {
        Some((page, slug))
            if !slug.is_empty() && !slug.contains('/') && page.contains("-chapter-") =>
        {
            Ok(format!("/comic/{slug}/{page}"))
        }
        _ => Err(source_error(format!("malformed chapter id {}", chapter.0))),
    }
}

/// The page images of a reader page (its `#sv-data` JSON).
pub fn parse_reader_page(chapter: &ChapterId, html: &str) -> Result<PageUrls> {
    let data = script_json(html, "sv-data")
        .ok_or_else(|| Error::NotFound(format!("chapter {chapter}")))?;
    let reader: ReaderData = json(data)?;
    Ok(PageUrls(
        reader.chapter.images.into_iter().map(|i| i.url).collect(),
    ))
}

/// Numbers arrive as strings ("12.5") or, defensively, JSON numbers.
fn text(raw: Option<&Value>) -> String {
    match raw {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
