//! Reading a manga and its chapters for a volume export from the library.

use crate::domain::{Chapter, Error, Manga, MangaId, Result};
use crate::library::{self, Library};

/// A manga and the chapters picked for export, in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeSource {
    pub manga: Manga,
    pub chapters: Vec<Chapter>,
}

/// `source`/`manga_id` from the library with its chapters, limited to `chapter_ids` when given
/// (reading order is kept, not the order of the ids).
pub fn load(
    lib: &Library,
    source: &str,
    manga_id: &str,
    chapter_ids: Option<&[String]>,
) -> Result<VolumeSource> {
    let id = MangaId(manga_id.to_string());
    let manga = library::get_manga(lib, source, &id)?
        .ok_or_else(|| Error::NotFound(format!("{manga_id} is not in your library")))?;
    let chapters = library::list_chapters(lib, source, &id, None)?
        .into_iter()
        .filter(|c| chapter_ids.is_none_or(|ids| ids.contains(&c.id.0)))
        .collect();
    Ok(VolumeSource { manga, chapters })
}
