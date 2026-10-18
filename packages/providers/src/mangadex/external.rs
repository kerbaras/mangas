//! Externally hosted MangaDex chapters (`externalUrl`, e.g. MANGA Plus): which ones are listed
//! and the link the app may open for them.

use super::api::ChapterData;
use mangas_core::domain::Availability;

pub(super) fn is_listed(c: &ChapterData) -> bool {
    let a = &c.attributes;
    !a.is_unavailable && (external_url(c).is_some() || a.pages != Some(0))
}

fn external_url(c: &ChapterData) -> Option<&str> {
    let url = c.attributes.external_url.as_deref()?.trim();
    (!url.is_empty()).then_some(url)
}

pub(super) fn availability(c: &ChapterData) -> Availability {
    match external_url(c) {
        Some(url) => Availability::External {
            url: secure_link(url),
        },
        None => Availability::Available,
    }
}

/// The link to open for an externally hosted chapter: a well-formed `https` URL with a host,
/// `http` upgraded to `https` (every site MangaDex links to serves it), anything else no link.
pub fn secure_link(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() == "http" {
        parsed.set_scheme("https").ok()?;
    }
    let has_host = parsed.host_str().is_some_and(|h| !h.is_empty());
    (parsed.scheme() == "https" && has_host).then(|| parsed.into())
}
