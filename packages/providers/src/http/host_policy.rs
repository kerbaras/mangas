use mangas_core::sources::Header;
use url::Url;

use crate::session::Session;

/// Which headers a request gets. Page images, by host: the site itself (and its subdomains, same
/// scheme) gets the referer and the browser session; declared image hosts get the referer
/// only, over https; any other host gets nothing. Credentials never leave the site. Requests
/// the client sends to its own base: see [`HostPolicy::request_headers`].
#[derive(Debug, Clone)]
pub struct HostPolicy {
    scheme: String,
    host: String,
    /// The site's port: the session and referer stay off other services on the same host.
    port: Option<u16>,
    referer: String,
    origin: String,
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
            port: parsed.as_ref().and_then(Url::port_or_known_default),
            referer: format!("{}/", site.trim_end_matches('/')),
            origin: site.trim_end_matches('/').to_string(),
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
        let on_site = !self.host.is_empty()
            && url.scheme() == self.scheme
            && url.port_or_known_default() == self.port
            && host_matches(host, &self.host);
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

    /// Headers of a request sent to the client's own base. All of them carry the site's referer:
    /// Cloudflare site rules challenge API calls that do not look like they come from the site.
    /// `/api` paths are the site's own `fetch()` calls, so they also carry the origin and, with a
    /// browser session, the fetch metadata the webview sends.
    pub fn request_headers(&self, url: &str) -> Vec<Header> {
        let mut headers = vec![("referer".to_string(), self.referer.clone())];
        if is_api_path(url) {
            headers.push(("origin".to_string(), self.origin.clone()));
            if self.session.is_some() {
                headers.extend(
                    SAME_ORIGIN_FETCH
                        .iter()
                        .map(|(name, value)| (name.to_string(), value.to_string())),
                );
            }
        }
        headers
    }
}

/// What a browser sends with a `fetch()` from the site's own page.
const SAME_ORIGIN_FETCH: [(&str, &str); 3] = [
    ("sec-fetch-site", "same-origin"),
    ("sec-fetch-mode", "cors"),
    ("sec-fetch-dest", "empty"),
];

/// The URL's path is `/api` or under it.
pub fn is_api_path(url: &str) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|u| u.path_segments()?.next().map(|s| s == "api"))
        .unwrap_or(false)
}

/// `host` is `pattern` or one of its subdomains (case-insensitive).
pub fn host_matches(host: &str, pattern: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let pattern = pattern.trim_start_matches('.').to_ascii_lowercase();
    !pattern.is_empty() && (host == pattern || host.ends_with(&format!(".{pattern}")))
}
