use clap::Args as ClapArgs;
use mangas_core::domain::{Manga, MangaId, MangaStatus};
use mangas_core::library;

use super::source_by_name;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (see `mangas search` or `mangas list`)
    pub id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
}

/// The stored record when the manga is followed, otherwise the source's details.
pub async fn run(args: Args) -> anyhow::Result<()> {
    let id = MangaId(args.id);
    let lib = library::open(&paths::library_db()?)?;
    let manga = match library::get_manga(&lib, &args.source, &id)? {
        Some(stored) => stored,
        None => source_by_name(&args.source)?.manga(&id).await?,
    };
    println!("{}", describe(&manga));
    Ok(())
}

/// "ongoing", "completed", ...; empty when the source doesn't say.
pub fn status_label(status: MangaStatus) -> String {
    match status {
        MangaStatus::Unknown => String::new(),
        known => format!("{known:?}").to_lowercase(),
    }
}

/// Title, then one labelled line per known field, then the description.
pub fn describe(m: &Manga) -> String {
    let status = status_label(m.status);
    let year = m.year.map(|y| y.to_string()).unwrap_or_default();
    let fields = [
        ("source", format!("{} {}", m.source, m.id)),
        ("status", status),
        ("authors", m.authors.join(", ")),
        ("artists", m.artists.join(", ")),
        ("year", year),
        ("tags", m.tags.join(", ")),
    ];
    let mut out = m.title.clone();
    for (label, value) in fields.iter().filter(|(_, v)| !v.is_empty()) {
        out.push_str(&format!("\n  {label:<8} {value}"));
    }
    if !m.description.is_empty() {
        out.push_str(&format!("\n\n{}", m.description));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mangas_core::domain::MangaStatus;

    #[test]
    fn lists_known_fields_and_skips_empty_ones() {
        let m = Manga {
            id: MangaId("solo".into()),
            source: "comick".into(),
            title: "Solo Leveling".into(),
            description: "E-rank hunter.".into(),
            status: MangaStatus::Completed,
            authors: vec!["Chugong".into(), "h-goon".into()],
            year: Some(2018),
            ..Default::default()
        };
        assert_eq!(
            describe(&m),
            "Solo Leveling\n  source   comick solo\n  status   completed\n  \
             authors  Chugong, h-goon\n  year     2018\n\nE-rank hunter."
        );
    }

    #[test]
    fn an_unknown_status_has_no_line() {
        let m = Manga {
            title: "Boruto".into(),
            source: "mangaplus".into(),
            id: MangaId("100269".into()),
            ..Default::default()
        };
        assert_eq!(describe(&m), "Boruto\n  source   mangaplus 100269");
    }
}
