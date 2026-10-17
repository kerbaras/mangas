use mangas_core::sources::Header;
use url::Url;

use crate::session::Session;

/// Which headers a page image request gets, by host: the site itself (and its subdomains, same
/// scheme) gets the referer and the browser session; declared image hosts get the referer
/// only, over https; any other host gets nothing. Credentials never leave the site.
#[derive(Debug, Clone)]
pub struct HostPolicy {
    scheme: String,
    host: String,
    referer: String,
    image_hosts: &'static [&'static str],
    session: Option<Session>,
}

impl HostPolicy {
    /// `site` is the site's origin (`https://example.com`).
    pub fn new(site: &str, image_hosts: &'static [&'static str], session: Option<Session>) -> Self {
        let parsed = Url::parse(site).ok();
        Self {
            scheme: parsed.as_ref().map_or("https", Url::scheme).to_string(),
            host: parsed
                .as_ref()
                .and_then(Url::host_str)
                .unwrap_or_default()
                .to_string(),
            referer: format!("{}/", site.trim_end_matches('/')),
            image_hosts,
            session,
        }
    }

    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn headers_for(&self, url: &str) -> Vec<Header> {
        let Ok(url) = Url::parse(url) else {
            return Vec::new();
        };
        let host = url.host_str().unwrap_or_default();
        let on_site =
            !self.host.is_empty() && url.scheme() == self.scheme && host_matches(host, &self.host);
        let image_host = url.scheme() == "https"
            && self
                .image_hosts
                .iter()
                .any(|pattern| *pattern == "*" || host_matches(host, pattern));
        let mut headers = Vec::new();
        if on_site || image_host {
            headers.push(("referer".to_string(), self.referer.clone()));
        }
        if let Some(session) = self.session.as_ref().filter(|_| on_site) {
            headers.extend(session.headers());
        }
        headers
    }
}

/// `host` is `pattern` or one of its subdomains (case-insensitive).
pub fn host_matches(host: &str, pattern: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let pattern = pattern.trim_start_matches('.').to_ascii_lowercase();
    !pattern.is_empty() && (host == pattern || host.ends_with(&format!(".{pattern}")))
}
