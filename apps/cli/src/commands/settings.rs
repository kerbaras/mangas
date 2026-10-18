use clap::{Args as ClapArgs, Subcommand};
use mangas_core::library;

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Show or set the preferred chapter languages, most preferred first (shared with the app)
    Languages(LanguagesArgs),
}

#[derive(ClapArgs)]
struct LanguagesArgs {
    /// Languages to save, e.g. "es-la,en"; prints the current ones when omitted
    list: Option<String>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    match args.action {
        Action::Languages(args) => languages(args),
    }
}

fn languages(args: LanguagesArgs) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let mut settings = library::get_settings(&lib)?;
    if let Some(list) = args.list {
        let parsed = library::parse_languages(&list);
        anyhow::ensure!(!parsed.is_empty(), "no language in '{list}'");
        settings.preferred_languages = parsed;
        library::set_settings(&lib, &settings)?;
    }
    outln!("{}", settings.languages().join(","))?;
    Ok(())
}
