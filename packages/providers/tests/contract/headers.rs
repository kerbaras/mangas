//! Page headers follow the `HostPolicy`: the referer where the provider needs it, session
//! credentials only on the site's own host.

use mangas_core::library::ProviderSession;
use mangas_providers::http::host_matches;
use mangas_providers::{Header, ProviderInfo, Source};
use url::Url;

pub fn assert_page_headers(
    source: &dyn Source,
    info: &ProviderInfo,
    pages: &[String],
    need_referer: bool,
    session: Option<&ProviderSession>,
) {
    let id = info.id;
    let referer = format!("{}/", info.site.trim_end_matches('/'));
    let site_host = Url::parse(info.site)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_default();
    for page in pages {
        let headers = source.page_headers(page);
        let got = get(&headers, "referer");
        assert_eq!(
            got.is_some(),
            need_referer,
            "{id}: referer on {page}: {got:?}"
        );
        if let Some(got) = got {
            assert_eq!(got, referer, "{id}: referer on {page}");
        }
        if get(&headers, "cookie").is_some() {
            let host = Url::parse(page)
                .ok()
                .and_then(|u| u.host_str().map(str::to_string));
            let on_site = host.is_some_and(|h| host_matches(&h, &site_host));
            assert!(on_site, "{id}: session cookie sent to {page}");
        }
    }

    let own = source.page_headers(&format!("{referer}contract/001.webp"));
    assert_eq!(
        get(&own, "referer"),
        Some(referer.as_str()),
        "{id}: referer on the site"
    );
    if let Some(session) = session.filter(|_| info.needs_browser_session) {
        assert_eq!(
            get(&own, "cookie"),
            Some(session.cookies.as_str()),
            "{id}: cookie"
        );
        let agent = get(&own, "user-agent");
        assert_eq!(agent, Some(session.user_agent.as_str()), "{id}: user agent");
    }

    let foreign = source.page_headers("https://contract.invalid/001.webp");
    assert_eq!(
        get(&foreign, "cookie"),
        None,
        "{id}: cookie leaked to a foreign host"
    );
    assert_eq!(get(&foreign, "user-agent"), None, "{id}: session UA leaked");
}

fn get<'a>(headers: &'a [Header], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}
