//! Redirects followed by hand, one hop at a time, so each hop goes through the destination
//! host's token bucket and headers ([`super::HostPolicy`]) instead of reqwest's built-in policy.

use reqwest::header::LOCATION;
use reqwest::{Method, Response, Url};

/// Hops after the first request before giving up.
pub const MAX_REDIRECTS: usize = 10;

/// The request a redirect asks for: where, with which method, and whether the body goes along.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hop {
    pub method: Method,
    pub url: Url,
    pub keep_body: bool,
}

/// The next request after `status` with `location`, answered to `method` at `from`; `None` when
/// the response is not a redirect to follow. Per RFC 9110, 303 turns anything but HEAD into a
/// GET, 301/302 turn a POST into a GET (what browsers do), and 307/308 repeat the request as is.
/// Only `http(s)` targets are followed.
pub fn next_hop(method: &Method, status: u16, location: Option<&str>, from: &Url) -> Option<Hop> {
    let url = from.join(location?.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let to_get = match status {
        301 | 302 => method == Method::POST,
        303 => method != Method::HEAD,
        307 | 308 => false,
        _ => return None,
    };
    Some(Hop {
        method: if to_get { Method::GET } else { method.clone() },
        url,
        keep_body: !to_get,
    })
}

/// [`next_hop`] for a received response.
pub fn hop_for(method: &Method, response: &Response) -> Option<Hop> {
    let location = response.headers().get(LOCATION)?.to_str().ok();
    next_hop(method, response.status().as_u16(), location, response.url())
}

/// The same `host:port` (scheme included) as `other`: a hop that stays there keeps the
/// request's own headers.
pub fn same_origin(url: &Url, other: &Url) -> bool {
    url.origin() == other.origin()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from() -> Url {
        Url::parse("https://site.example/api/a?x=1").unwrap()
    }

    #[test]
    fn post_becomes_get_on_301_302_303_but_not_307_308() {
        for (status, method, keep) in [
            (301, Method::GET, false),
            (302, Method::GET, false),
            (303, Method::GET, false),
            (307, Method::POST, true),
            (308, Method::POST, true),
        ] {
            let hop = next_hop(&Method::POST, status, Some("/b"), &from()).unwrap();
            assert_eq!((hop.method, hop.keep_body), (method, keep), "{status}");
        }
    }

    #[test]
    fn get_stays_get_and_head_stays_head() {
        let hop = next_hop(&Method::GET, 301, Some("/b"), &from()).unwrap();
        assert_eq!(hop.method, Method::GET);
        let hop = next_hop(&Method::HEAD, 303, Some("/b"), &from()).unwrap();
        assert_eq!(hop.method, Method::HEAD);
    }

    #[test]
    fn relative_and_absolute_locations_resolve_against_the_response() {
        let hop = next_hop(&Method::GET, 302, Some("b?y=2"), &from()).unwrap();
        assert_eq!(hop.url.as_str(), "https://site.example/api/b?y=2");
        let hop = next_hop(&Method::GET, 302, Some("https://cdn.example/p"), &from()).unwrap();
        assert_eq!(hop.url.as_str(), "https://cdn.example/p");
    }

    #[test]
    fn non_redirects_missing_locations_and_other_schemes_are_final() {
        assert_eq!(next_hop(&Method::GET, 200, Some("/b"), &from()), None);
        assert_eq!(next_hop(&Method::GET, 304, Some("/b"), &from()), None);
        assert_eq!(next_hop(&Method::GET, 302, None, &from()), None);
        assert_eq!(
            next_hop(&Method::GET, 302, Some("file:///etc/passwd"), &from()),
            None
        );
    }
}
