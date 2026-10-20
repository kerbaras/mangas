//! Which source a command means for a manga id when `--source` is left out: the one the
//! library follows it on, like the desktop app, which always knows a title's source.

use anyhow::bail;
use mangas_core::domain::MangaId;
use mangas_core::library::{self, Library};

use super::hint;

/// The provider `--source` falls back to for a title the library does not follow.
pub const DEFAULT_SOURCE: &str = "mangadex";

/// The source of the followed manga `id`: `given` when it follows the title there, else the
/// only source that follows it. Not followed, or followed on several sources with none given,
/// is an error that says so.
pub fn followed_source(lib: &Library, id: &MangaId, given: Option<&str>) -> anyhow::Result<String> {
    let sources = library::followed_sources(lib, id)?;
    match (given, sources.as_slice()) {
        (Some(given), _) if sources.iter().any(|s| s == given) => Ok(given.to_string()),
        (None, [only]) => Ok(only.clone()),
        (None, []) => {
            let add = hint(&format!("add {id}"), DEFAULT_SOURCE);
            bail!("{id} is not in your library; run {add} first (or pass --source)")
        }
        (None, many) => bail!(
            "{id} is followed on {}; pick one with --source",
            many.join(" and ")
        ),
        (Some(given), []) => {
            let add = hint(&format!("add {id}"), given);
            bail!("{id} is not in your library; run {add} first")
        }
        (Some(given), others) => bail!(
            "{id} is not followed on {given}; your library has it on {} (pass that to --source)",
            others.join(" and ")
        ),
    }
}

/// `given`, else [`followed_source`], else [`DEFAULT_SOURCE`] when no source follows `id`: for
/// commands that also work on titles the library does not follow.
pub fn source_or_default(
    lib: &Library,
    id: &MangaId,
    given: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(given) = given {
        return Ok(given.to_string());
    }
    match library::followed_sources(lib, id)?.is_empty() {
        true => Ok(DEFAULT_SOURCE.to_string()),
        false => followed_source(lib, id, None),
    }
}
