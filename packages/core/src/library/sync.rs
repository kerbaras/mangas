//! Refreshing a manga's chapter lists from its source, one request per language.

use crate::domain::{Chapter, Error, MangaId, Result};
use crate::library::{upsert_chapters, Library};
use crate::sources::Source;

/// Chapter lists of `id` per language, in the order asked. A language the source fails on is
/// logged and skipped; the call only fails when every language does (with the first error).
pub async fn fetch_chapters(
    source: &dyn Source,
    id: &MangaId,
    languages: &[String],
) -> Result<Vec<(String, Vec<Chapter>)>> {
    let mut lists = Vec::new();
    let mut first_error = None;
    for lang in languages {
        match source.chapters(id, lang).await {
            Ok(chapters) => lists.push((lang.clone(), chapters)),
            Err(e) => {
                tracing::warn!("{} {id}: {lang} chapters not synced: {e}", source.name());
                first_error.get_or_insert(e);
            }
        }
    }
    match (lists.is_empty(), first_error) {
        (true, Some(e)) => Err(e),
        (true, None) => Err(Error::Other("no chapter language given".into())),
        _ => Ok(lists),
    }
}

/// [`fetch_chapters`] and store each list in the library; returns every fetched chapter.
pub async fn sync_chapters(
    lib: &Library,
    source: &dyn Source,
    id: &MangaId,
    languages: &[String],
) -> Result<Vec<Chapter>> {
    let lists = fetch_chapters(source, id, languages).await?;
    store_chapters(lib, source.name(), id, lists)
}

/// Stores per-language lists from [`fetch_chapters`]; returns all their chapters.
pub fn store_chapters(
    lib: &Library,
    source: &str,
    id: &MangaId,
    lists: Vec<(String, Vec<Chapter>)>,
) -> Result<Vec<Chapter>> {
    for (lang, chapters) in &lists {
        upsert_chapters(lib, source, id, lang, chapters)?;
    }
    Ok(lists.into_iter().flat_map(|(_, c)| c).collect())
}
