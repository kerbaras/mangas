use clap::Args as ClapArgs;
use mangas_core::domain::{Chapter, MangaId};
use mangas_core::library;

use super::source_by_name;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source
    pub id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
    /// Chapter language
    #[arg(long, default_value = "en")]
    pub lang: String,
    /// Read from the library instead of asking the source
    #[arg(long)]
    pub offline: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let id = MangaId(args.id);
    let lib = library::open(&paths::library_db()?)?;
    let followed = library::get_manga(&lib, &args.source, &id)?.is_some();
    let chapters = if args.offline {
        library::list_chapters(&lib, &args.source, &id, Some(&args.lang))?
    } else {
        let source = source_by_name(&args.source)?;
        let fresh = source.chapters(&id, &args.lang).await?;
        if followed {
            library::upsert_chapters(&lib, source.name(), &fresh)?;
            library::list_chapters(&lib, &args.source, &id, Some(&args.lang))?
        } else {
            fresh
        }
    };
    if chapters.is_empty() {
        println!(
            "No {} chapters hosted on {} for {id}",
            args.lang, args.source
        );
        return Ok(());
    }
    println!(
        "{:<8} {:<5} {:<3} {:<36}  title",
        "chapter", "vol", "dl", "id"
    );
    for c in &chapters {
        println!("{}", format_row(c));
    }
    if !followed {
        println!("\n(not in your library; `mangas add {id}` to follow it)");
    }
    Ok(())
}

fn format_row(c: &Chapter) -> String {
    format!(
        "{:<8} {:<5} {:<3} {:<36}  {}",
        c.number,
        c.volume.as_deref().unwrap_or("-"),
        if c.is_downloaded() { "yes" } else { "-" },
        c.id,
        c.title.as_deref().unwrap_or("")
    )
}
