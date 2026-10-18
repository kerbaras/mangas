//! `/series/{id}`: title (`h1`), cover (`og:image`), the "Status:" and "Description" entries.
//! Unknown ids redirect to `/404`, a normal page without a series `h1`.

use mangas_core::domain::{Error, Manga, MangaId, MangaStatus, Result};

use super::SOURCE_NAME;
use crate::html::{self, Document, Element};

pub fn parse_series(id: &MangaId, body: &str) -> Result<Manga> {
    let doc = html::parse(body);
    let title = html::first(html::root(&doc), "main h1")?
        .map(html::text)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Error::NotFound(format!("{SOURCE_NAME} series {id}")))?;
    let status = field(&doc, "Status")?
        .and_then(|li| html::first(li, "a").ok().flatten())
        .map(html::text)
        .map_or(MangaStatus::Unknown, |s| parse_status(&s));
    let description = field(&doc, "Description")?
        .and_then(|li| html::first(li, "p").ok().flatten())
        .map(html::text_lines)
        .unwrap_or_default();
    Ok(Manga {
        id: id.clone(),
        source: SOURCE_NAME.to_string(),
        title,
        description,
        cover_url: html::meta(&doc, "og:image")?,
        status,
    })
}

/// The `<li>` whose leading `<strong>` reads `label` (with or without a trailing colon).
fn field<'a>(doc: &'a Document, label: &str) -> Result<Option<Element<'a>>> {
    for li in html::select(html::root(doc), "li")? {
        let Some(strong) = html::first(li, "strong")? else {
            continue;
        };
        if html::text(strong).trim_end_matches(':').trim() == label {
            return Ok(Some(li));
        }
    }
    Ok(None)
}

pub fn parse_status(raw: &str) -> MangaStatus {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ongoing" => MangaStatus::Ongoing,
        "complete" | "completed" => MangaStatus::Completed,
        "hiatus" => MangaStatus::Hiatus,
        "canceled" | "cancelled" => MangaStatus::Cancelled,
        _ => MangaStatus::Unknown,
    }
}

/// The id in `/series/{id}` or `/series/{id}/{slug}` (absolute or relative link).
pub fn series_id(href: &str) -> Option<String> {
    segment_after(href, "series")
}

/// The path segment following `marker` in `href`.
pub fn segment_after(href: &str, marker: &str) -> Option<String> {
    let path = href.split(['?', '#']).next().unwrap_or_default();
    let mut segments = path.split('/').skip_while(|s| *s != marker).skip(1);
    segments
        .next()
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(str::to_string)
}
