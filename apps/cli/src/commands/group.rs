//! `mangas group <id> [name]`: list a followed title's scanlation groups, or narrow the title
//! to one so Details, `mangas download` and auto-download take only its chapters.

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::{groups_of, normalize_group, MangaId};
use mangas_core::library;

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
    #[arg(long, default_value = "mangadex")]
    pub source: String,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let id = MangaId(args.id);
    let lib = library::open(&paths::library_db()?)?;
    if library::get_manga(&lib, &args.source, &id)?.is_none() {
        bail!("{id} is not in your library; run `mangas add {id}` first");
    }
    let groups = groups_of(&library::list_chapters(&lib, &args.source, &id, None)?);
    if args.clear {
        library::set_chapter_group(&lib, &args.source, &id, None)?;
        outln!("{id}: every group")?;
        return Ok(());
    }
    if let Some(name) = normalize_group(args.name.as_deref()) {
        if !groups.iter().any(|g| g.name == name) {
            bail!("{id} has no chapters of group \"{name}\"; `mangas group {id}` lists them");
        }
        library::set_chapter_group(&lib, &args.source, &id, Some(&name))?;
        outln!("{id}: only chapters of {name}")?;
        return Ok(());
    }
    let saved = library::chapter_group(&lib, &args.source, &id)?;
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
        None => outln!("\nEvery group shown (`mangas group {id} <name>` narrows it)")?,
    }
    Ok(())
}
