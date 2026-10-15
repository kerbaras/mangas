use clap::Args as ClapArgs;
use mangas_core::domain::Manga;
use mangas_core::library::{self, Library};

use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Print one line per manga as `source id title`
    #[arg(long)]
    pub plain: bool,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let all = library::list_manga(&lib)?;
    if all.is_empty() {
        println!("Your library is empty. Try: mangas search <title>");
        return Ok(());
    }
    for m in all {
        if args.plain {
            println!("{} {} {}", m.source, m.id, m.title);
        } else {
            println!(
                "{:<40} {:<10} {:<36} {}",
                truncate(&m.title, 40),
                m.source,
                m.id,
                progress(&lib, &m)?
            );
        }
    }
    Ok(())
}

fn progress(lib: &Library, m: &Manga) -> anyhow::Result<String> {
    let chapters = library::list_chapters(lib, &m.source, &m.id)?;
    let downloaded = chapters.iter().filter(|c| c.is_downloaded()).count();
    Ok(format!("{downloaded}/{} downloaded", chapters.len()))
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let cut: String = s.chars().take(n - 1).collect();
    format!("{cut}…")
}
