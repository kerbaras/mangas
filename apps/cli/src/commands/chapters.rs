use clap::Args as ClapArgs;
use mangas_core::domain::{sort_by_number, Availability, Chapter, MangaId};
use mangas_core::library;

use super::{source_by_name, stored_settings};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source
    pub id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
    /// Chapter languages, e.g. "es,en" [default: your preferred languages]
    #[arg(long)]
    pub lang: Option<String>,
    /// Read from the library instead of asking the source
    #[arg(long)]
    pub offline: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let id = MangaId(args.id);
    let lib = library::open(&paths::library_db()?)?;
    let followed = library::get_manga(&lib, &args.source, &id)?.is_some();
    let requested = library::requested_languages(args.lang.as_deref(), &stored_settings());
    let languages = mangas_providers::languages_for(&args.source, &requested);
    let stored = || library::list_chapters(&lib, &args.source, &id, None);
    // Imported manga have no provider: their chapters only live in the library.
    let chapters = if args.offline || library::is_local(&args.source) {
        library::in_languages(stored()?, &languages)
    } else {
        let source = source_by_name(&args.source)?;
        if followed {
            library::sync_chapters(&lib, source.as_ref(), &id, &languages).await?;
            library::in_languages(stored()?, &languages)
        } else {
            let lists = library::fetch_chapters(source.as_ref(), &id, &languages).await?;
            sort_by_number(lists.into_iter().flat_map(|(_, c)| c).collect())
        }
    };
    if chapters.is_empty() {
        let langs = languages.join("/");
        outln!("No {langs} chapters hosted on {} for {id}", args.source)?;
        return Ok(());
    }
    outln!(
        "{:<8} {:<5} {:<8} {:<6} {:<36}  title",
        "chapter",
        "vol",
        "status",
        "lang",
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
        "{:<8} {:<5} {:<8} {:<6} {:<36}  {title}",
        c.number,
        c.volume.as_deref().unwrap_or("-"),
        status(c),
        c.language,
        c.id,
    )
}

/// `exported`, `on disk`, `locked`, `external` or `-` (downloadable, not downloaded yet).
fn status(c: &Chapter) -> &'static str {
    match (&c.availability, c.is_downloaded()) {
        (_, true) if c.is_exported() => "exported",
        (_, true) => "on disk",
        (Availability::Locked, false) => "locked",
        (Availability::External { .. }, false) => "external",
        (Availability::Available, false) => "-",
    }
}
