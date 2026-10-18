use clap::Args as ClapArgs;
use mangas_core::sources::search::group;
use mangas_core::sources::{ProviderStatus, SearchGroup};

use super::{source_by_name, stored_settings};
use crate::output::out;

#[derive(ClapArgs)]
pub struct Args {
    /// Title to search for
    pub query: String,
    /// Provider to search; every enabled provider when omitted
    #[arg(long, visible_alias = "source")]
    pub provider: Option<String>,
    /// Results per provider
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let groups = match &args.provider {
        Some(id) => {
            let source = source_by_name(id)?;
            vec![group(id, source.search(&args.query, args.limit).await)]
        }
        None => {
            mangas_providers::search_enabled(&stored_settings(), &args.query, args.limit, &[]).await
        }
    };
    out!("{}", render(&groups, &args.query, name_of))?;
    if !groups.is_empty() && groups.iter().all(|g| !g.is_ok()) {
        anyhow::bail!("no provider could search for \"{}\"", args.query);
    }
    Ok(())
}

fn name_of(id: &str) -> String {
    mangas_providers::info(id).map_or_else(|| id.to_string(), |p| p.name.to_string())
}

/// One block per provider: a header with its name and count or problem, then its results.
fn render(groups: &[SearchGroup], query: &str, name: impl Fn(&str) -> String) -> String {
    if groups.is_empty() {
        return "No providers enabled: turn one on in the app (Settings > Providers)\n".into();
    }
    let mut out = String::new();
    for g in groups {
        let header = match &g.status {
            ProviderStatus::Ready if g.items.is_empty() => format!("no results for \"{query}\""),
            ProviderStatus::Ready => format!(
                "{} result{}",
                g.items.len(),
                if g.items.len() == 1 { "" } else { "s" }
            ),
            ProviderStatus::NeedsSession => {
                "needs a session (connect in the app: Settings > Providers)".into()
            }
            ProviderStatus::Unreachable { reason } => format!("unreachable ({reason})"),
        };
        out.push_str(&format!("{}: {header}\n", name(&g.provider)));
        for m in &g.items {
            let status = format!("{:?}", m.status).to_lowercase();
            out.push_str(&format!("  {:<36}  {status:<10}  {}\n", m.id, m.title));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mangas_core::domain::{Error, Manga, MangaId, MangaStatus};

    fn manga(id: &str, title: &str) -> Manga {
        Manga {
            id: MangaId(id.into()),
            source: "a".into(),
            title: title.into(),
            description: String::new(),
            cover_url: None,
            status: MangaStatus::Ongoing,
            ..Default::default()
        }
    }

    #[test]
    fn groups_print_under_their_provider() {
        let groups = [
            group("a", Ok(vec![manga("1", "Berserk")])),
            group("b", Ok(Vec::new())),
            group(
                "c",
                Err(Error::Challenge {
                    name: "c".into(),
                    site: "s".into(),
                }),
            ),
            group("d", Err(Error::Other("HTTP 503".into()))),
        ];
        let out = render(&groups, "berserk", |id| id.to_uppercase());
        let lines: Vec<_> = out.lines().map(str::trim_end).collect();
        assert_eq!(lines[0], "A: 1 result");
        assert!(lines[1].starts_with("  1 ") && lines[1].ends_with("ongoing     Berserk"));
        assert_eq!(lines[2], "B: no results for \"berserk\"");
        assert!(lines[3].starts_with("C: needs a session"));
        assert_eq!(lines[4], "D: unreachable (HTTP 503)");
    }

    #[test]
    fn no_enabled_provider_says_so() {
        assert!(render(&[], "x", str::to_string).starts_with("No providers enabled"));
    }
}
