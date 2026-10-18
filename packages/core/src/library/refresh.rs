use super::{get_manga, upsert_chapters, upsert_manga, Library};
use crate::domain::{Chapter, MangaId, Result};
use crate::sources::Source;

/// What [`refresh_from_source`] did with the chapters it fetched.
#[derive(Debug, Clone, PartialEq)]
pub enum Refreshed {
    /// The title is followed: its chapter list (and record, when available) were saved.
    Saved,
    /// The title is not followed: nothing was stored, here is the source's list.
    NotFollowed(Vec<Chapter>),
}

/// Re-syncs a title from its source. For a followed title the manga record is saved again too,
/// so fields added after it was followed (authors, year, tags…) get filled on refresh. The
/// record is best effort: if only it fails, the fresh chapters are still saved.
pub async fn refresh_from_source(
    lib: &Library,
    source: &dyn Source,
    id: &MangaId,
    language: &str,
) -> Result<Refreshed> {
    let chapters = source.chapters(id, language).await?;
    if !is_followed(lib, source, id)? {
        return Ok(Refreshed::NotFollowed(chapters));
    }
    let manga = source.manga(id).await;
    // Checked again after the awaits: an unfollow meanwhile must not be undone by this write.
    if !is_followed(lib, source, id)? {
        return Ok(Refreshed::NotFollowed(chapters));
    }
    upsert_chapters(lib, source.name(), id, language, &chapters)?;
    match manga {
        Ok(manga) => upsert_manga(lib, &manga)?,
        Err(e) => tracing::warn!("{}: kept the saved record of {id}: {e}", source.name()),
    }
    Ok(Refreshed::Saved)
}

fn is_followed(lib: &Library, source: &dyn Source, id: &MangaId) -> Result<bool> {
    Ok(get_manga(lib, source.name(), id)?.is_some())
}
