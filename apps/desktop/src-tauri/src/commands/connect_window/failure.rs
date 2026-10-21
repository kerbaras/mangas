//! What a failed Connect-window load means, from each webview's own error codes. Pure, so the
//! macOS and Windows mappings are tested on every platform.

use mangas_core::domain::{system_has_root_certificates, webview_tls_failure, CertProblem};

/// One failed main-frame load, in the webview's terms.
#[derive(Debug, PartialEq, Eq)]
pub enum Load {
    Certificate(CertProblem),
    Network(String),
    /// A load the app stopped itself: a blocked navigation or a page replaced by the next one.
    Ignored,
}

/// The message the UI shows for a failed load of `uri`, the same on every platform; `None` for
/// a load the app stopped itself.
pub fn reason(uri: &str, load: Load) -> Option<String> {
    match load {
        Load::Ignored => None,
        Load::Certificate(problem) => Some(webview_tls_failure(
            &host_of(uri),
            &problem,
            system_has_root_certificates(),
        )),
        Load::Network(message) => Some(format!("network: couldn't load {uri}: {message}")),
    }
}

fn host_of(uri: &str) -> String {
    url::Url::parse(uri)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| uri.to_string())
}

/// WebView2's `COREWEBVIEW2_WEB_ERROR_STATUS`, from a `NavigationCompleted` that failed.
#[cfg(any(windows, test))]
pub fn webview2(status: i32) -> Load {
    let net = |text: &str| Load::Network(text.to_string());
    match status {
        1 => Load::Certificate(CertProblem::WrongHost),
        2 => Load::Certificate(CertProblem::Expired),
        3 => Load::Certificate(CertProblem::Other("client certificate refused".into())),
        4 => Load::Certificate(CertProblem::Revoked),
        5 => Load::Certificate(CertProblem::UnknownIssuer),
        6 => net("the server is unreachable"),
        7 => net("the connection timed out"),
        8 => net("the server sent an invalid response"),
        9 => net("the connection was aborted"),
        10 => net("the connection was reset"),
        11 => net("the internet connection was lost"),
        12 => net("couldn't connect to the server"),
        13 => net("the host name couldn't be resolved"),
        14 => Load::Ignored,
        15 => net("a redirect failed"),
        17 | 18 => net("the server asked for credentials"),
        other => Load::Network(format!("unknown error (WebView2 status {other})")),
    }
}

/// WKWebView's `NSError` from `webView:didFailProvisionalNavigation:withError:`.
#[cfg(any(target_os = "macos", test))]
pub fn wkwebview(domain: &str, code: isize, description: &str) -> Load {
    match (domain, code) {
        ("NSURLErrorDomain", -999) | ("WebKitErrorDomain", 102) => Load::Ignored,
        ("NSURLErrorDomain", -1201) => Load::Certificate(CertProblem::Expired),
        ("NSURLErrorDomain", -1202 | -1203) => Load::Certificate(CertProblem::UnknownIssuer),
        ("NSURLErrorDomain", -1204) => Load::Certificate(CertProblem::NotYetValid),
        ("NSURLErrorDomain", -1200 | -1205 | -1206) => {
            Load::Certificate(CertProblem::Other(description.to_string()))
        }
        _ => Load::Network(description.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_certificate_failure_reads_the_same_on_every_platform() {
        let uri = "https://comick.live/";
        let windows = reason(uri, webview2(5));
        let macos = reason(uri, wkwebview("NSURLErrorDomain", -1202, "untrusted"));
        let linux = reason(uri, Load::Certificate(CertProblem::UnknownIssuer));
        assert_eq!(windows, linux);
        assert_eq!(macos, linux);
        if system_has_root_certificates() {
            let text = linux.unwrap_or_default();
            assert!(text.contains("the certificate of comick.live couldn't be verified"));
        }
        assert_eq!(webview2(2), Load::Certificate(CertProblem::Expired));
        assert_eq!(webview2(1), Load::Certificate(CertProblem::WrongHost));
    }

    #[test]
    fn dns_failures_name_the_page_and_cancels_are_ignored() {
        let dns = reason("https://nope.invalid/", webview2(13));
        assert_eq!(
            dns.as_deref(),
            Some(
                "network: couldn't load https://nope.invalid/: the host name couldn't be resolved"
            )
        );
        let mac = wkwebview(
            "NSURLErrorDomain",
            -1003,
            "A server with the specified hostname could not be found.",
        );
        assert!(matches!(mac, Load::Network(_)));
        assert_eq!(reason("https://x/", webview2(14)), None);
        assert_eq!(
            reason(
                "https://x/",
                wkwebview("NSURLErrorDomain", -999, "cancelled")
            ),
            None
        );
        assert_eq!(
            reason(
                "https://x/",
                wkwebview("WebKitErrorDomain", 102, "interrupted")
            ),
            None
        );
    }
}
