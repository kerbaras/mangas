//! Pure mapping from MangaDot wire types to domain types.

use mangas_core::domain::{
    sort_by_number, Chapter, ChapterId, Error, Manga, MangaId, MangaStatus, Result,
};
use mangas_core::sources::PageUrls;

use super::api::{ChapterEntry, ChapterImages, MangaDetail, MangaEntry, MangaList};
use super::metadata;

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
        authors: metadata::names(entry.authors.as_ref()),
        artists: metadata::names(entry.artists.as_ref()),
        year: metadata::year(entry.year.as_ref()),
        tags: metadata::tags(entry.genres.as_ref(), entry.tags.as_ref()),
    }
}

pub fn parse_manga_list(body: &str) -> Result<Vec<Manga>> {
    let list: MangaList = json(body)?;
    Ok(list.manga_list.into_iter().map(to_manga).collect())
}

/// `/api/manga/{id}`: the record sits under `manga`, `null` when the id does not exist.
pub fn parse_manga_detail(id: &MangaId, body: &str) -> Result<Manga> {
    let detail: MangaDetail = json(body)?;
    detail
        .manga
        .map(to_manga)
        .ok_or_else(|| Error::NotFound(format!("manga {id}")))
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

/// Label of the chapters the site mirrors itself (`source: "scraper"`, no group). Not
/// "Official": MangaDot has a real group of that name.
pub const MIRROR_GROUP: &str = "MangaDot mirror";

/// Who released `entry`: the credited groups ("A, B"), else the group record, else the
/// uploader's free-text scanlator name; mirrored chapters get [`MIRROR_GROUP`].
pub fn group(entry: &ChapterEntry) -> String {
    let clean = |s: Option<&str>| s.map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    let mut names: Vec<String> = Vec::new();
    for name in entry.groups.iter().filter_map(|g| clean(g.name.as_deref())) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if !names.is_empty() {
        return names.join(", ");
    }
    clean(entry.group_name.as_deref())
        .or_else(|| clean(entry.scanlator_name.as_deref()))
        .unwrap_or_else(|| MIRROR_GROUP.to_string())
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
            exported_at: None,
            group: Some(group(e)),
            availability: Default::default(),
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
