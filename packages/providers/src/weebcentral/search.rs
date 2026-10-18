//! `/search/data` (HTMX fragment, "Minimal Display"): one `<article>` per series with a link
//! to `/series/{id}/{slug}`, the title and a "type · year · status" line.

use std::collections::HashSet;

use mangas_core::domain::{Manga, MangaId, MangaStatus, Result};

use super::series::{parse_status, series_id};
use super::SOURCE_NAME;
use crate::html::{self, Element};

/// Covers live on the site's image CDN under the series id; the series page links the same
/// `fallback` JPEG in its `og:image`.
const COVER_BASE: &str = "https://temp.compsci88.com/cover/fallback";

pub fn cover_url(id: &str) -> String {
    format!("{COVER_BASE}/{id}.jpg")
}

pub fn parse_search(body: &str) -> Result<Vec<Manga>> {
    let doc = html::parse(body);
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for article in html::select(html::root(&doc), "article")? {
        let Some(manga) = to_manga(article)? else {
            continue;
        };
        if seen.insert(manga.id.clone()) {
            found.push(manga);
        }
    }
    Ok(found)
}

fn to_manga(article: Element<'_>) -> Result<Option<Manga>> {
    let Some(link) = html::first(article, r#"a[href*="/series/"]"#)? else {
        return Ok(None);
    };
    let Some(id) = html::attr(link, "href").and_then(|href| series_id(&href)) else {
        return Ok(None);
    };
    let title = match html::first(link, "h2")? {
        Some(h2) => html::text(h2),
        None => html::attr(link, "data-tip").unwrap_or_default(),
    };
    if title.is_empty() {
        return Ok(None);
    }
    let status = html::select(article, "section div")?
        .into_iter()
        .map(html::text)
        .map(|s| parse_status(&s))
        .find(|s| *s != MangaStatus::Unknown)
        .unwrap_or(MangaStatus::Unknown);
    Ok(Some(Manga {
        cover_url: Some(cover_url(&id)),
        id: MangaId(id),
        source: SOURCE_NAME.to_string(),
        title,
        description: String::new(),
        status,
    }))
}
