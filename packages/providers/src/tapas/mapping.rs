//! Pure mapping from Tapas pages and the episodes JSON to domain types.

use mangas_core::domain::{
    sort_by_number, Availability, Chapter, ChapterId, Error, Manga, MangaId, MangaStatus, Result,
};
use mangas_core::sources::PageUrls;

use super::api::{Episode, EpisodesResponse};
use super::html::{attr, clean_text, meta, tags, text_after, texts_of_class};

pub const SOURCE_NAME: &str = "tapas";
pub const SITE: &str = "https://tapas.io";

fn source_error(message: impl Into<String>) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message: message.into(),
    }
}

/// `GET /search?q=..&t=COMICS` -> one manga per `li.search-item-wrap`.
pub fn parse_search(html: &str) -> Vec<Manga> {
    let mut found: Vec<Manga> = Vec::new();
    for item in html.split("class=\"search-item-wrap\"").skip(1) {
        let item = item.split("</li>").next().unwrap_or(item);
        let Some(manga) = search_item(item) else {
            continue;
        };
        if !found.iter().any(|m| m.id == manga.id) {
            found.push(manga);
        }
    }
    found
}

fn search_item(item: &str) -> Option<Manga> {
    let id = tags(item, "a")
        .into_iter()
        .find_map(|t| attr(t, "data-series-id"))
        .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))?;
    let title_section = &item[item.find("class=\"title-section\"")?..];
    let title = text_after(title_section, "<a ", "</a>")?;
    Some(Manga {
        id: MangaId(id),
        source: SOURCE_NAME.to_string(),
        title,
        description: text_after(item, "class=\"desc", "</p>").unwrap_or_default(),
        cover_url: tags(item, "img").first().and_then(|t| attr(t, "src")),
        status: MangaStatus::Unknown,
        ..Default::default()
    })
}

/// `GET /series/{id}/info` -> the series: creators as authors, genres as tags. The page shows
/// no publication status or year.
pub fn parse_series(id: &MangaId, html: &str) -> Result<Manga> {
    let title = text_after(html, "<a class=\"title\"", "</a>")
        .or_else(|| meta(html, "og:site_name").map(|t| t.trim_start_matches("Read ").to_string()))
        .ok_or_else(|| Error::NotFound(format!("tapas series {id}")))?;
    Ok(Manga {
        id: id.clone(),
        source: SOURCE_NAME.to_string(),
        title,
        description: text_after(html, "class=\"description__body\"", "</span>").unwrap_or_default(),
        cover_url: meta(html, "og:image"),
        status: MangaStatus::Unknown,
        authors: texts_of_class(html, "creator__item"),
        tags: texts_of_class(html, "genre-btn"),
        ..Default::default()
    })
}

/// One page of the episode list: its episodes and whether another page follows.
pub fn parse_episodes(body: &str) -> Result<(Vec<Episode>, bool)> {
    let response: EpisodesResponse = serde_json::from_str(body)
        .map_err(|e| source_error(format!("unexpected episodes JSON: {e}")))?;
    Ok((response.data.episodes, response.data.pagination.has_next))
}

/// The episode-list page to fetch after `page`, or `None` once the list is complete. A list
/// still going after `cap` pages is an error: a truncated list would read as complete and the
/// library would drop the chapters missing from it.
pub fn next_episode_page(
    manga: &MangaId,
    page: u32,
    has_next: bool,
    batch_len: usize,
    cap: u32,
) -> Result<Option<u32>> {
    if !has_next || batch_len == 0 {
        Ok(None)
    } else if page >= cap {
        Err(source_error(format!(
            "series {manga} has more than {cap} episode pages"
        )))
    } else {
        Ok(Some(page + 1))
    }
}

/// Every episode as a chapter, numbered by `scene`. Paid, wait-until-free and early access
/// episodes cannot be read without an account, so they are listed as locked.
pub fn to_chapters(episodes: &[Episode], manga: &MangaId) -> Vec<Chapter> {
    let chapters: Vec<Chapter> = episodes
        .iter()
        .map(|e| Chapter {
            id: ChapterId(e.id.to_string()),
            manga_id: manga.clone(),
            title: e.title.as_deref().map(clean_text).filter(|t| !t.is_empty()),
            language: "en".to_string(),
            volume: None,
            number: match (e.scene, e.pending_scene) {
                (0, 0) => String::new(),
                (0, pending) => pending.to_string(),
                (scene, _) => scene.to_string(),
            },
            local_path: None,
            group: None,
            availability: if e.is_free() {
                Availability::Available
            } else {
                Availability::Locked
            },
        })
        .collect();
    sort_by_number(chapters)
}

/// `GET /episode/{id}` -> the `img.content__img` URLs. A locked episode renders no images (or
/// redirects to sign-in), which is an error rather than an empty chapter.
pub fn parse_pages(chapter: &ChapterId, html: &str) -> Result<PageUrls> {
    let pages: Vec<String> = tags(html, "img")
        .into_iter()
        .filter(|t| attr(t, "class").is_some_and(|c| c.split(' ').any(|c| c == "content__img")))
        .filter_map(|t| attr(t, "data-src").or_else(|| attr(t, "src")))
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
        .collect();
    if pages.is_empty() {
        return Err(source_error(format!(
            "episode {chapter} has no free pages: it is locked (paid, wait-until-free or early access)"
        )));
    }
    Ok(PageUrls(pages))
}
