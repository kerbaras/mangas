//! Pure mapping from MANGA Plus replies (protobuf bytes) to domain types.

use mangas_core::domain::{
    sort_by_number, Availability, Chapter, ChapterId, Manga, MangaId, Result,
};
use mangas_core::sources::PageUrls;

use super::page::page_url;
use super::proto::{Chapter as ApiChapter, Title, TitleDetailView};
use super::response::{decode, error};
use super::status::status;
use super::SOURCE_NAME;

/// API language numbers, by position: 0 en, 1 es, ... 9 vi (8 is unused).
const LANGUAGES: [&str; 10] = ["en", "es", "fr", "id", "pt-br", "ru", "th", "de", "", "vi"];

pub fn language_code(language: i32) -> Option<&'static str> {
    let code = LANGUAGES.get(usize::try_from(language).ok()?)?;
    (!code.is_empty()).then_some(*code)
}

/// `title_list/allV2` searched like the site does: every title in `languages` whose name or
/// author contains `query` (case-insensitive), in the API's order.
pub fn parse_search(
    body: &[u8],
    query: &str,
    languages: &[String],
    limit: usize,
) -> Result<Vec<Manga>> {
    let view = decode(body, "title list")?.all_titles_view_v2;
    let query = query.trim().to_lowercase();
    let wanted = |t: &Title| {
        let language = language_code(t.language);
        t.title_id != 0
            && language.is_some_and(|l| languages.iter().any(|w| w == l))
            && (t.name.to_lowercase().contains(&query) || t.author.to_lowercase().contains(&query))
    };
    let mut found: Vec<Manga> = Vec::new();
    let titles = view
        .into_iter()
        .flat_map(|v| v.all_titles_group)
        .flat_map(|g| g.titles);
    for title in titles.filter(wanted) {
        if !found.iter().any(|m| m.id.0 == title.title_id.to_string()) {
            found.push(to_manga(&title));
        }
    }
    found.truncate(limit.max(1));
    Ok(found)
}

/// The details of title `id` (`title_detailV3`).
pub fn parse_title(body: &[u8], id: &MangaId) -> Result<Manga> {
    let detail = detail(body, id)?;
    let title = detail.title.clone().unwrap_or_default();
    let description = [&detail.overview, &detail.viewing_period_description]
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(Manga {
        id: id.clone(),
        description,
        status: status(&detail),
        tags: detail.genre_list.iter().map(|g| g.name.clone()).collect(),
        ..to_manga(&title)
    })
}

/// The free chapters of title `id` (the first and the latest ones), sorted by number. A title
/// is published in one language; asking for another is an empty list.
pub fn parse_chapters(body: &[u8], id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
    let detail = detail(body, id)?;
    let own = detail
        .title
        .as_ref()
        .and_then(|t| language_code(t.language));
    if own != Some(language) {
        return Ok(Vec::new());
    }
    let mut chapters: Vec<Chapter> = Vec::new();
    let listed = detail.chapter_list_group.iter().flat_map(|g| {
        let first = g.first_chapter_list.iter();
        first.chain(g.last_chapter_list.iter())
    });
    for chapter in listed {
        if !chapters
            .iter()
            .any(|c| c.id.0 == chapter.chapter_id.to_string())
        {
            chapters.push(to_chapter(chapter, id, language));
        }
    }
    Ok(sort_by_number(chapters))
}

/// The page image URLs of a chapter (`manga_viewer_v3`), each carrying its key and the
/// viewer's token ([`page_url`]).
pub fn parse_viewer(body: &[u8]) -> Result<PageUrls> {
    let viewer = decode(body, "viewer")?.manga_viewer.unwrap_or_default();
    let token = viewer.view_token;
    let pages: Vec<String> = (viewer.pages.into_iter())
        .filter_map(|p| p.manga_page)
        .filter(|p| !p.image_url.is_empty())
        .map(|p| page_url(&p.image_url, &p.encryption_key, &token))
        .collect();
    if pages.is_empty() {
        return Err(error("viewer: chapter without pages".to_string()));
    }
    Ok(PageUrls(pages))
}

/// "#012" -> "12", "#1100.5" -> "1100.5"; "ex" or "One-shot" -> "".
pub fn chapter_number(name: &str) -> String {
    let digits = name.trim().trim_start_matches('#');
    if digits.is_empty() || digits.parse::<f64>().is_err() {
        return String::new();
    }
    match digits.trim_start_matches('0') {
        "" => "0".to_string(),
        n if n.starts_with('.') => format!("0{n}"),
        n => n.to_string(),
    }
}

fn detail(body: &[u8], id: &MangaId) -> Result<TitleDetailView> {
    let what = format!("title {id}");
    decode(body, &what)?
        .title_detail_view
        .ok_or_else(|| error(format!("{what}: no details")))
}

fn to_manga(title: &Title) -> Manga {
    Manga {
        id: MangaId(title.title_id.to_string()),
        source: SOURCE_NAME.to_string(),
        title: title.name.trim().to_string(),
        cover_url: Some(title.portrait_image_url.clone()).filter(|u| !u.is_empty()),
        authors: (title.author.split('/'))
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect(),
        ..Manga::default()
    }
}

/// A chapter without a subtitle has expired (no longer free to read).
fn to_chapter(chapter: &ApiChapter, id: &MangaId, language: &str) -> Chapter {
    Chapter {
        id: ChapterId(chapter.chapter_id.to_string()),
        manga_id: id.clone(),
        title: chapter.sub_title.clone().filter(|s| !s.trim().is_empty()),
        language: language.to_string(),
        volume: None,
        number: chapter_number(&chapter.name),
        local_path: None,
        group: None,
        availability: match chapter.sub_title {
            Some(_) => Availability::Available,
            None => Availability::Locked,
        },
        exported_at: None,
    }
}
