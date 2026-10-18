//! Pure mapping from Webtoons responses to domain types.

use mangas_core::domain::{
    season_label, sort_by_number, Chapter, Error, Manga, MangaId, MangaStatus, Result,
};
use mangas_core::sources::PageUrls;

use super::api::{EpisodePage, EpisodesReply, SearchReply};
use super::html::{canvas_cards, creators, genre, is_motion_toon, og, unescape, viewer_images};
use super::ids::{Episode, Title};
use super::{INFO, SOURCE_NAME};
use crate::resolve_languages;

/// Thumbnail host that serves covers without a referer (`webtoon-phinf` answers 403).
const COVER_HOST: &str = "swebtoon-phinf.pstatic.net";
const IMAGE_HOST: &str = "webtoon-phinf.pstatic.net";

fn source_error(message: impl Into<String>) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message: message.into(),
    }
}

/// Cover URL loadable by a plain `<img>`: CDN paths become absolute on the referer-free host.
pub fn cover_url(raw: &str) -> String {
    if raw.starts_with('/') {
        format!("https://{COVER_HOST}{raw}")
    } else {
        raw.replacen(
            &format!("://{IMAGE_HOST}/"),
            &format!("://{COVER_HOST}/"),
            1,
        )
    }
}

fn manga(title: &Title, name: String, cover: Option<String>, description: String) -> Manga {
    Manga {
        id: title.id(),
        source: SOURCE_NAME.to_string(),
        title: name,
        description,
        cover_url: cover.as_deref().map(cover_url),
        ..Default::default()
    }
}

/// Originals from the search autocomplete JSON.
pub fn parse_search(body: &str, language: &str) -> Result<Vec<Manga>> {
    let reply: SearchReply =
        serde_json::from_str(body).map_err(|e| source_error(format!("search: {e}")))?;
    let entries = reply.result.map(|r| r.searched_list).unwrap_or_default();
    Ok(entries
        .into_iter()
        .filter_map(|e| {
            let (number, name) = (e.title_no?, unescape(&e.title?));
            let by = e.author_name_list.join(", ");
            let description = if by.is_empty() {
                by
            } else {
                format!("By {by}")
            };
            let title = Title::original(language, number);
            let authors = e.author_name_list.iter().map(|a| unescape(a)).collect();
            Some(Manga {
                authors,
                ..manga(&title, name, e.thumbnail_mobile, description)
            })
        })
        .filter(|m| !m.title.is_empty())
        .collect())
}

/// Canvas titles from the canvas search page.
pub fn parse_canvas_search(html: &str, language: &str) -> Vec<Manga> {
    canvas_cards(html)
        .into_iter()
        .map(|c| {
            manga(
                &Title::canvas(language, c.number),
                c.title,
                c.cover,
                String::new(),
            )
        })
        .collect()
}

/// Title page (`og:*` tags, the update badge, the creators line and the genre). Keeps the id
/// it was asked for.
pub fn parse_title_page(id: &MangaId, html: &str) -> Result<Manga> {
    let name = og(html, "og:title").ok_or_else(|| Error::NotFound(format!("webtoon {id}")))?;
    Ok(Manga {
        id: id.clone(),
        source: SOURCE_NAME.to_string(),
        title: name,
        description: og(html, "og:description").unwrap_or_default(),
        cover_url: og(html, "og:image").as_deref().map(cover_url),
        status: title_status(html),
        authors: creators(html),
        tags: genre(html).into_iter().collect(),
        ..Default::default()
    })
}

/// `COMPLETED` badge vs `UP every …` schedule; canvas pages show neither.
pub fn title_status(html: &str) -> MangaStatus {
    if html.contains("txt_ico_completed") {
        MangaStatus::Completed
    } else if html.contains("txt_ico_up") {
        MangaStatus::Ongoing
    } else {
        MangaStatus::Unknown
    }
}

/// One page of the episode list; an unknown title has a null `result`.
pub fn parse_episode_page(body: &str, title: &Title) -> Result<EpisodePage> {
    let reply: EpisodesReply =
        serde_json::from_str(body).map_err(|e| source_error(format!("episodes: {e}")))?;
    reply
        .result
        .ok_or_else(|| Error::NotFound(format!("webtoon {}", title.id())))
}

/// Episodes as chapters numbered by `episodeNo` (sequential on the site), sorted.
/// The episode title shown to readers: season markers become `S2 Ep. 5` (see [`season_label`]);
/// the chapter number stays the site's absolute episode index for ordering and ranges.
fn episode_title(raw: &str) -> Option<String> {
    let title = unescape(raw);
    let title = title.trim();
    (!title.is_empty()).then(|| season_label(title).unwrap_or_else(|| title.to_string()))
}

pub fn to_chapters(manga: &MangaId, title: &Title, pages: Vec<EpisodePage>) -> Vec<Chapter> {
    let chapters = pages
        .into_iter()
        .flat_map(|p| p.episode_list)
        .map(|e| Chapter {
            id: Episode {
                title: title.clone(),
                number: e.episode_no,
            }
            .id(),
            manga_id: manga.clone(),
            title: e.episode_title.as_deref().and_then(episode_title),
            language: title.language.clone(),
            volume: None,
            number: e.episode_no.to_string(),
            local_path: None,
            exported_at: None,
            group: None,
            availability: Default::default(),
        })
        .collect();
    let mut chapters = sort_by_number(chapters);
    chapters.dedup_by(|a, b| a.id == b.id);
    chapters
}

/// Page images of a viewer page; motion toons (animated layers) are not supported.
pub fn parse_viewer(html: &str) -> Result<PageUrls> {
    let pages = viewer_images(html);
    if pages.is_empty() {
        let why = if is_motion_toon(html) {
            "motion webtoons are not supported"
        } else {
            "no page images in the viewer"
        };
        return Err(source_error(why));
    }
    Ok(PageUrls(pages))
}

/// `preferred` matched to the editions Webtoons has (a regional code reads its base edition,
/// `es-la` -> `es`), keeping the preference order; English when none of them is published.
pub fn supported(preferred: &[String]) -> Vec<String> {
    let editions = resolve_languages(preferred, INFO.languages);
    if editions.is_empty() {
        vec![INFO.languages[0].to_string()]
    } else {
        editions
    }
}
