use std::sync::Arc;

use mangas_core::domain::Result;
use mangas_core::library::Settings;
use mangas_core::sources::Source;
use serde::Serialize;

/// What a provider can do; apps skip providers lacking what a screen needs (e.g. search).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Search,
    Chapters,
    Download,
}

/// What the apps need to list a provider and decide whether to offer "Connect".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProviderInfo {
    /// Stable id stored with every manga (`Source::name`).
    pub id: &'static str,
    pub name: &'static str,
    /// Origin of the site (`https://example.com`): referer, Connect window, challenge message.
    pub site: &'static str,
    /// Chapter languages offered (ISO 639-1, MangaDex-style regional suffixes).
    pub languages: &'static [&'static str],
    pub capabilities: &'static [Capability],
    /// The site sits behind a browser verification; a stored session is required.
    pub needs_browser_session: bool,
    pub icon_url: Option<&'static str>,
}

/// A registered provider: its description and how to build its [`Source`] from the settings
/// (browser session, preferences).
pub trait Provider: Send + Sync {
    fn info(&self) -> ProviderInfo;
    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>>;
}
