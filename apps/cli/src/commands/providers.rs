use clap::Args as ClapArgs;
use mangas_providers::{ProviderInfo, ProviderStatus};

use super::stored_settings;

#[derive(ClapArgs)]
pub struct Args {}

/// Lists the providers with their status: one cheap request each, using the browser session
/// the desktop app stored (connect from Settings in the app).
pub async fn run(_args: Args) -> anyhow::Result<()> {
    for (info, status) in mangas_providers::statuses(&stored_settings()).await {
        println!("{}", line(&info, &status));
    }
    Ok(())
}

fn line(info: &ProviderInfo, status: &ProviderStatus) -> String {
    let hint = match status {
        ProviderStatus::NeedsSession => " (connect in the app: Settings > Providers)",
        _ => "",
    };
    format!(
        "{:<10} {:<10} {:<24} {status}{hint}",
        info.id, info.name, info.site
    )
}
