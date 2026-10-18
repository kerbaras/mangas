use clap::Args as ClapArgs;
use mangas_core::domain::{
    in_group, normalize_group, sort_by_number, Availability, Chapter, MangaId,
};
use mangas_core::library;

use super::{hint, source_by_name, stored_settings, unknown_group};
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
        return Err(unknown_group(group, &id, &args.source));
    }
    if chapters.is_empty() {
        let langs = languages.join("/");
        outln!("No {langs} chapters hosted on {} for {id}", args.source)?;
        return Ok(());
    }
    let id_width = id_width(&chapters);
    outln!(
        "{:<8} {:<5} {:<9} {:<6} {:<GROUP_WIDTH$} {:<id_width$}  title",
        "chapter",
        "vol",
        "status",
        "lang",
        "group",
        "id"
    )?;
    for c in &chapters {
        outln!("{}", format_row(c, id_width))?;
    }
    if !followed {
        let add = hint(&format!("add {id}"), &args.source);
        outln!("\n(not in your library; {add} to follow it)")?;
    }
    Ok(())
}

/// The id column fits the longest id (MangaDex UUIDs, `en-webtoon-95-653`, `u:1833853`).
fn id_width(chapters: &[Chapter]) -> usize {
    chapters
        .iter()
        .map(|c| c.id.0.chars().count())
        .fold("id".len(), usize::max)
}

fn format_row(c: &Chapter, id_width: usize) -> String {
    let title = c.title.as_deref().unwrap_or("");
    let title = match &c.availability {
        Availability::External { url: Some(url) } => {
            format!("{title} (read at {url})").trim().to_string()
        }
        _ => title.to_string(),
    };
    format!(
        "{:<8} {:<5} {:<9} {:<6} {:<GROUP_WIDTH$} {:<id_width$}  {title}",
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

#[cfg(test)]
mod tests {
    use super::*;
    use mangas_core::domain::ChapterId;

    fn chapter(id: &str, title: &str) -> Chapter {
        Chapter {
            id: ChapterId(id.into()),
            number: "1".into(),
            title: Some(title.into()),
            language: "en".into(),
            ..Default::default()
        }
    }

    #[test]
    fn titles_start_in_one_column_whatever_the_id_length() {
        let rows = [
            chapter("en-webtoon-95-1", "S1 Ep. 0"),
            chapter("en-webtoon-95-653", "S3 Ep. 235"),
            chapter("u:1833853", "Chapter 0"),
        ];
        let width = id_width(&rows);
        assert_eq!(width, "en-webtoon-95-653".len());
        let starts: Vec<usize> = rows
            .iter()
            .map(|c| {
                let title = c.title.as_deref().unwrap_or_default();
                format_row(c, width).find(title).unwrap_or_default()
            })
            .collect();
        assert!(
            starts.iter().all(|s| *s == starts[0] && *s > 0),
            "{starts:?}"
        );
    }

    #[test]
    fn an_id_longer_than_a_uuid_widens_the_column() {
        let long = "x".repeat(50);
        assert_eq!(id_width(&[chapter(&long, "t")]), 50);
        assert_eq!(id_width(&[]), 2);
    }

    #[test]
    fn ids_honour_width_like_strings() {
        assert_eq!(format!("{:<5}|", ChapterId("ab".into())), "ab   |");
        assert_eq!(format!("{:<5}|", MangaId("ab".into())), "ab   |");
    }
}
