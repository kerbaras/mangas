//! Which release of each chapter `mangas download` takes when a title lists several.

use mangas_core::domain::{normalize_group, same_slot, Chapter, MangaId};
use mangas_core::download::queue;
use mangas_core::library::{Library, Settings};

use super::hint;
use crate::output::outln;

/// Every release with `--all-groups`; else the title's group, or one release per chapter (the
/// one on disk first, then the preferred groups).
pub fn pick(
    lib: &Library,
    source: &str,
    synced: &[Chapter],
    group: Option<&str>,
    all_groups: bool,
    settings: &Settings,
) -> anyhow::Result<Vec<Chapter>> {
    if all_groups {
        return Ok(synced.to_vec());
    }
    let preferred = settings.preferred_groups(source);
    Ok(queue::pick_releases(
        lib,
        source,
        synced.to_vec(),
        group,
        &preferred,
    )?)
}

/// Says so when another release of a selected chapter was left out.
pub fn report_left_out(
    synced: &[Chapter],
    selected: &[Chapter],
    id: &MangaId,
    source: &str,
    settings: &Settings,
) -> anyhow::Result<()> {
    let other = |s: &Chapter| synced.iter().any(|r| r.id != s.id && same_slot(r, s));
    if !selected.iter().any(other) {
        return Ok(());
    }
    let by = if settings.preferred_groups(source).is_empty() {
        "the one on disk first; no preferred groups set"
    } else {
        "the one on disk first, then preferred groups"
    };
    let save = hint(&format!("group {id} <name>"), source);
    outln!(
        "Took one release per chapter ({by}); --all-groups takes every release, {save} saves a group"
    )?;
    Ok(())
}

/// `--group` wins over the title's saved group; `--all-groups` drops it.
pub fn pick_group(
    explicit: Option<&str>,
    all_groups: bool,
    saved: Option<String>,
) -> Option<String> {
    match normalize_group(explicit) {
        Some(group) => Some(group),
        None if all_groups => None,
        None => saved,
    }
}

#[cfg(test)]
mod tests {
    use super::pick_group;

    #[test]
    fn explicit_group_overrides_the_saved_one() {
        let saved = || Some("Official".to_string());
        assert_eq!(
            pick_group(Some("K Manga"), false, saved()).as_deref(),
            Some("K Manga")
        );
        assert_eq!(
            pick_group(None, false, saved()).as_deref(),
            Some("Official")
        );
        assert_eq!(pick_group(None, true, saved()), None);
        assert_eq!(pick_group(Some(" "), false, None), None);
    }
}
