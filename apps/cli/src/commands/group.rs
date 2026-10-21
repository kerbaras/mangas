//! `mangas group <id> [name]`: list a followed title's scanlation groups, or narrow the title
//! to one so Details, `mangas download` and auto-download take only its chapters.

use clap::Args as ClapArgs;
use mangas_core::domain::{groups_of, normalize_group, MangaId};
use mangas_core::library;

use super::followed::followed_source;
use super::{hint, unknown_group};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (must be followed)
    pub id: String,
    /// Group to narrow the title to (the name `mangas chapters` shows)
    #[arg(conflicts_with = "clear")]
    pub name: Option<String>,
    /// Show every group again
    #[arg(long)]
    pub clear: bool,
    /// Source of the title [default: the one your library follows it on]
    #[arg(long)]
    pub source: Option<String>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let id = MangaId(args.id);
    let lib = library::open(&paths::library_db()?)?;
    let source = followed_source(&lib, &id, args.source.as_deref())?;
    let groups = groups_of(&library::list_chapters(&lib, &source, &id, None)?);
    if args.clear {
        library::set_chapter_group(&lib, &source, &id, None)?;
        outln!("{id}: every group")?;
        return Ok(());
    }
    if let Some(name) = normalize_group(args.name.as_deref()) {
        if !groups.iter().any(|g| g.name == name) {
            return Err(unknown_group(&name, &id, &source));
        }
        library::set_chapter_group(&lib, &source, &id, Some(&name))?;
        outln!("{id}: only chapters of {name}")?;
        return Ok(());
    }
    let saved = library::chapter_group(&lib, &source, &id)?;
    if groups.is_empty() {
        outln!("No groups credited for {id}")?;
    }
    for g in &groups {
        let mark = if saved.as_deref() == Some(g.name.as_str()) {
            "*"
        } else {
            " "
        };
        outln!("{mark} {:>5}  {}", g.chapters, g.name)?;
    }
    match saved {
        Some(name) => outln!("\nNarrowed to {name} (`--clear` shows every group)")?,
        None => {
            let narrow = hint(&format!("group {id} <name>"), &source);
            outln!("\nEvery group shown ({narrow} narrows it)")?
        }
    }
    Ok(())
}
