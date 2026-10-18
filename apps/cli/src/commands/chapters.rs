use clap::Args as ClapArgs;
use mangas_core::domain::{Availability, Chapter, MangaId};
use mangas_core::library;

use super::source_by_name;
use crate::output::outln;
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
    // Imported manga have no provider: their chapters only live in the library.
    let chapters = if args.offline || library::is_local(&args.source) {
        library::list_chapters(&lib, &args.source, &id, Some(&args.lang))?
    } else {
        let source = source_by_name(&args.source)?;
        let fresh = source.chapters(&id, &args.lang).await?;
        if followed {
            library::upsert_chapters(&lib, source.name(), &id, &args.lang, &fresh)?;
            library::list_chapters(&lib, &args.source, &id, Some(&args.lang))?
        } else {
            fresh
        }
    };
    if chapters.is_empty() {
        outln!(
            "No {} chapters hosted on {} for {id}",
            args.lang,
            args.source
        )?;
        return Ok(());
    }
    outln!(
        "{:<8} {:<5} {:<8} {:<36}  title",
        "chapter",
        "vol",
        "status",
        "id"
    )?;
    for c in &chapters {
        outln!("{}", format_row(c))?;
    }
    if !followed {
        outln!("\n(not in your library; `mangas add {id}` to follow it)")?;
    }
    Ok(())
}

fn format_row(c: &Chapter) -> String {
    let title = c.title.as_deref().unwrap_or("");
    let title = match &c.availability {
        Availability::External { url: Some(url) } => {
            format!("{title} (read at {url})").trim().to_string()
        }
        _ => title.to_string(),
    };
    format!(
        "{:<8} {:<5} {:<8} {:<36}  {title}",
        c.number,
        c.volume.as_deref().unwrap_or("-"),
        status(c),
        c.id,
    )
}

/// `on disk`, `locked`, `external` or `-` (downloadable, not downloaded yet).
fn status(c: &Chapter) -> &'static str {
    match (&c.availability, c.is_downloaded()) {
        (_, true) => "on disk",
        (Availability::Locked, false) => "locked",
        (Availability::External { .. }, false) => "external",
        (Availability::Available, false) => "-",
    }
}
