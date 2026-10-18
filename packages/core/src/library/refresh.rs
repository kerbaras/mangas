use super::{get_manga, upsert_chapters, upsert_manga, Library};
use crate::domain::{Chapter, MangaId, Result};
use crate::sources::Source;

/// What [`refresh_from_source`] did with the chapters it fetched.
#[derive(Debug, Clone, PartialEq)]
pub enum Refreshed {
    /// The title is followed: its record and chapter list were saved to the library.
    Saved,
    /// The title is not followed: nothing was stored, here is the source's list.
    NotFollowed(Vec<Chapter>),
}

/// Re-syncs a title from its source. For a followed title the manga record is saved again too,
/// so fields added after it was followed (authors, year, tags…) get filled on refresh.
pub async fn refresh_from_source(
    lib: &Library,
    source: &dyn Source,
    id: &MangaId,
    language: &str,
) -> Result<Refreshed> {
    let chapters = source.chapters(id, language).await?;
    if get_manga(lib, source.name(), id)?.is_none() {
        return Ok(Refreshed::NotFollowed(chapters));
    }
    let manga = source.manga(id).await?;
    upsert_manga(lib, &manga)?;
    upsert_chapters(lib, source.name(), id, language, &chapters)?;
    Ok(Refreshed::Saved)
}
