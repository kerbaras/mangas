use clap::Args as ClapArgs;
use mangas_core::domain::{
    in_group, normalize_group, sort_by_number, Availability, Chapter, MangaId,
};
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
    /// Only the chapters of this scanlation group (the name in the group column)
    #[arg(long)]
    pub group: Option<String>,
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
    let group = normalize_group(args.group.as_deref());
    let chapters = in_group(chapters, group.as_deref());
    if let (Some(group), true) = (&group, chapters.is_empty()) {
        outln!("No chapters of group \"{group}\" for {id}; drop --group to list every group")?;
        return Ok(());
    }
    if chapters.is_empty() {
        let langs = languages.join("/");
        outln!("No {langs} chapters hosted on {} for {id}", args.source)?;
        return Ok(());
    }
    outln!(
        "{:<8} {:<5} {:<9} {:<6} {:<GROUP_WIDTH$} {:<36}  title",
        "chapter",
        "vol",
        "status",
        "lang",
        "group",
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
        "{:<8} {:<5} {:<9} {:<6} {:<GROUP_WIDTH$} {:<36}  {title}",
        c.number,
        c.volume.as_deref().unwrap_or("-"),
        status(c),
        c.language,
        group_cell(c.group.as_deref()),
        c.id,
    )
}

const GROUP_WIDTH: usize = 20;

/// The group name cut to the column, `-` when the source names none.
fn group_cell(group: Option<&str>) -> String {
    match group {
        None => "-".into(),
        Some(g) if g.chars().count() <= GROUP_WIDTH => g.into(),
        Some(g) => {
            let cut: String = g.chars().take(GROUP_WIDTH - 1).collect();
            format!("{cut}…")
        }
    }
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
