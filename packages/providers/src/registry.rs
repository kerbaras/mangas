use mangas_core::library::Settings;
use mangas_core::sources::Source;
use serde::Serialize;

use crate::session::Session;
use crate::{mangadex, mangadot};

/// What the apps need to list a provider and decide whether to offer "Connect".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProviderInfo {
    /// Stable id stored with every manga (`Source::name`).
    pub id: &'static str,
    pub name: &'static str,
    pub site: &'static str,
    /// The site sits behind a browser verification; a stored [`Session`] is required.
    pub needs_browser_session: bool,
}

pub const PROVIDERS: [ProviderInfo; 2] = [
    ProviderInfo {
        id: mangadex::SOURCE_NAME,
        name: "MangaDex",
        site: "https://mangadex.org",
        needs_browser_session: false,
    },
    ProviderInfo {
        id: mangadot::SOURCE_NAME,
        name: "MangaDot",
        site: mangadot::SITE,
        needs_browser_session: true,
    },
];

pub fn info(id: &str) -> Option<ProviderInfo> {
    PROVIDERS.iter().copied().find(|p| p.id == id)
}

/// The provider registered as `id`, configured with the session stored in `settings` if the
/// site needs one.
pub fn by_name(id: &str, settings: &Settings) -> Option<Box<dyn Source>> {
    let session = Session::from_settings(settings, id);
    match id {
        mangadex::SOURCE_NAME => mangadex::MangaDex::new()
            .ok()
            .map(|s| Box::new(s) as Box<dyn Source>),
        mangadot::SOURCE_NAME => mangadot::MangaDot::new(session)
            .ok()
            .map(|s| Box::new(s) as Box<dyn Source>),
        _ => None,
    }
}
