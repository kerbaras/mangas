use clap::Args as ClapArgs;
use mangas_providers::{Session, PROVIDERS};

use super::stored_settings;

#[derive(ClapArgs)]
pub struct Args {}

/// Lists the providers and, for sites behind a browser verification, whether the desktop app
/// has stored a session for them (`mangas` reuses it; connect from Settings in the app).
pub fn run(_args: Args) -> anyhow::Result<()> {
    let settings = stored_settings();
    for p in PROVIDERS {
        let state = if !p.needs_browser_session {
            "ready"
        } else if Session::from_settings(&settings, p.id).is_some() {
            "connected"
        } else {
            "not connected (open Settings in the app)"
        };
        println!("{:<10} {:<10} {:<24} {}", p.id, p.name, p.site, state);
    }
    Ok(())
}
