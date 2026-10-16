//! Pure mapping from MangaDot wire types to domain types.

use mangas_core::domain::{
    sort_by_number, Chapter, ChapterId, Error, Manga, MangaId, MangaStatus, Result,
};
use mangas_core::sources::PageUrls;

use super::api::{ChapterEntry, ChapterImages, MangaEntry, MangaList};

pub const SOURCE_NAME: &str = "mangadot";
pub const SITE: &str = "https://mangadot.net";

fn source_error(message: impl Into<String>) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message: message.into(),
    }
}

fn json<T: serde::de::DeserializeOwned>(body: &str) -> Result<T> {
    serde_json::from_str(body).map_err(|e| source_error(format!("unexpected response: {e}")))
}

/// Absolute URL for a path the API gives relative to the site (`/uploads/x.webp`).
pub fn site_url(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        format!("{SITE}/{}", path.trim_start_matches('/'))
    }
}

pub fn parse_status(raw: Option<&str>) -> MangaStatus {
    match raw.map(str::to_ascii_lowercase).as_deref() {
        Some("ongoing") => MangaStatus::Ongoing,
        Some("completed") => MangaStatus::Completed,
        Some("hiatus") => MangaStatus::Hiatus,
        Some("cancelled" | "canceled" | "dropped") => MangaStatus::Cancelled,
        _ => MangaStatus::Unknown,
    }
}

pub fn to_manga(entry: MangaEntry) -> Manga {
    Manga {
        id: MangaId(entry.id.to_string()),
        source: SOURCE_NAME.to_string(),
        title: entry.title,
        description: entry.description.unwrap_or_default(),
        cover_url: entry.photo.as_deref().map(site_url),
        status: parse_status(entry.status.as_deref()),
    }
}

pub fn parse_manga_list(body: &str) -> Result<Vec<Manga>> {
    let list: MangaList = json(body)?;
    Ok(list.manga_list.into_iter().map(to_manga).collect())
}

/// Open Graph tags of the `/manga/{id}` page (title, description, cover) plus the status from
/// the React Router loader data inlined in the page.
pub fn parse_manga_page(id: &MangaId, html: &str) -> Result<Manga> {
    let title = og(html, "og:title").ok_or_else(|| Error::NotFound(format!("manga {id}")))?;
    Ok(Manga {
        id: id.clone(),
        source: SOURCE_NAME.to_string(),
        title,
        description: og(html, "og:description").unwrap_or_default(),
        cover_url: og(html, "og:image"),
        status: parse_status(loader_status(html).as_deref()),
    })
}

/// The loader data is a flat turbo-stream array where the first occurrence of a value is
/// literal: `"status","Ongoing"`. Later occurrences are back-references, so the first literal
/// is the page's own manga. The array sits in a JS string, so quotes may arrive escaped.
pub fn loader_status(html: &str) -> Option<String> {
    let text = html.replace("\\\"", "\"");
    let key = "\"status\",\"";
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()))
}

fn og(html: &str, property: &str) -> Option<String> {
    let needle = format!("property=\"{property}\"");
    let tag_start = html.find(&needle)?;
    let tag = &html[tag_start..html[tag_start..].find('>')? + tag_start];
    let content = tag.find("content=\"")? + "content=\"".len();
    let value = &tag[content..tag[content..].find('"')? + content];
    Some(unescape(value)).filter(|v| !v.is_empty())
}

fn unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_string()
}

/// Numbers arrive as JSON numbers or strings ("2.0"); keep the shortest exact text.
pub fn number_text(raw: Option<&serde_json::Value>) -> String {
    match raw {
        Some(serde_json::Value::Number(n)) => trim_number(&n.to_string()),
        Some(serde_json::Value::String(s)) => trim_number(s.trim()),
        _ => String::new(),
    }
}

fn trim_number(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// Chapter ids carry the endpoint family: `c:<id>` for mirrored chapters, `u:<id>` for uploads.
pub fn chapter_id(entry: &ChapterEntry) -> ChapterId {
    let kind = if entry.source.as_deref() == Some("user") {
        'u'
    } else {
        'c'
    };
    ChapterId(format!("{kind}:{}", entry.id))
}

/// API path serving the images of `chapter`, from the id produced by [`chapter_id`].
pub fn images_path(chapter: &ChapterId) -> Result<String> {
    match chapter.0.split_once(':') {
        Some(("u", id)) => Ok(format!("/uploads/{id}/images")),
        Some(("c", id)) => Ok(format!("/chapters/{id}/images")),
        _ => Err(source_error(format!("malformed chapter id {}", chapter.0))),
    }
}

pub fn parse_chapters(body: &str, manga: &MangaId, language: &str) -> Result<Vec<Chapter>> {
    let entries: Vec<ChapterEntry> = json(body)?;
    let chapters = entries
        .iter()
        .filter(|e| e.language.as_deref().unwrap_or("en") == language)
        .map(|e| Chapter {
            id: chapter_id(e),
            manga_id: manga.clone(),
            title: e.chapter_title.clone().filter(|t| !t.trim().is_empty()),
            language: language.to_string(),
            volume: Some(number_text(e.volume_number.as_ref())).filter(|v| !v.is_empty()),
            number: number_text(e.chapter_number.as_ref()),
            local_path: None,
        })
        .collect();
    Ok(sort_by_number(chapters))
}

pub fn parse_images(body: &str) -> Result<PageUrls> {
    let images: ChapterImages = json(body)?;
    Ok(PageUrls(
        images.images.iter().map(|i| site_url(&i.url)).collect(),
    ))
}
