//! Provider list and the browser-session bridge for sites behind a verification wall: the
//! site opens in its own window, the user passes the check, and the resulting cookies are
//! stored with the user agent that obtained them for the Rust HTTP client to reuse.

use mangas_core::library::{self, ProviderSession, ProxyMode, Settings};
use mangas_providers::{ProviderInfo, ProviderStatus};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use super::{connect_window, open_library, settings_of, settings_or_default, to_ipc};

/// A current browser user agent for the engine the Connect window runs (Chromium on Windows,
/// WebKit elsewhere): Cloudflare's check loops when the two disagree. It binds `cf_clearance`
/// to this string, so the same one goes out with every API request that carries the cookie.
#[cfg(target_os = "windows")]
pub const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36";
#[cfg(target_os = "macos")]
pub const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
const CLEARANCE_COOKIE: &str = "cf_clearance";

#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    mangas_providers::providers()
}

/// Probes `provider` once with the stored session: ready, needs a session, or unreachable.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn provider_status(provider: String) -> Result<ProviderStatus, String> {
    let settings = settings_or_default();
    Ok(mangas_providers::status(&provider, &settings).await)
}

/// Probes MangaDex through the proxy being edited (`mode`, `url`), before it is saved.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn test_proxy(mode: ProxyMode, url: String) -> Result<ProviderStatus, String> {
    if mode == ProxyMode::Custom {
        library::proxy_url(&url).map_err(to_ipc)?;
    }
    let settings = Settings {
        proxy_mode: Some(mode),
        proxy_url: Some(url),
        ..settings_or_default()
    };
    Ok(mangas_providers::status("mangadex", &settings).await)
}

/// Whether downloads go through the proxy setting `mode`, `url` (passed in, so an edit still
/// being saved counts): a stored "non-public address" refusal may then download on a retry (#309).
#[tauri::command]
pub fn proxy_in_use(mode: ProxyMode, url: String) -> bool {
    let settings = Settings {
        proxy_mode: Some(mode),
        proxy_url: Some(url),
        ..settings_or_default()
    };
    mangas_providers::http::proxied(&settings.network())
}

/// Opens `provider`'s site in a private window, so only a clearance earned there is seen. Once
/// the verification cookie appears the session is saved and the window closes itself; closing it
/// or a page that fails to load ends the UI's wait (`connect_window`).
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn connect_provider(app: AppHandle, provider: String) -> Result<(), String> {
    let info = mangas_providers::info(&provider)
        .ok_or_else(|| to_ipc(format!("unknown provider '{provider}'")))?;
    let site = Url::parse(info.site).map_err(to_ipc)?;
    let label = format!("connect-{}", info.id);
    if let Some(existing) = app.get_webview_window(&label) {
        return existing.set_focus().map_err(to_ipc);
    }
    let id = info.id;
    let origin = site.clone();
    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(site.clone()))
        .title(format!("Connect {}", info.name))
        .inner_size(1000.0, 760.0)
        .incognito(true)
        .user_agent(BROWSER_USER_AGENT)
        .on_navigation(move |url| allowed_navigation(url, &origin))
        .on_page_load(move |webview, _| {
            if let Err(e) = capture_session(&webview, id, &site) {
                log::warn!("{id}: could not read session cookies: {e}");
            }
        })
        .build()
        .map_err(to_ipc)?;
    connect_window::watch(&window, id);
    Ok(())
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn disconnect_provider(provider: String) -> Result<(), String> {
    let lib = open_library()?;
    let mut settings = settings_of(&lib)?;
    settings.provider_sessions.remove(&provider);
    library::set_settings(&lib, &settings).map_err(to_ipc)
}

/// Saves the site cookies once the clearance cookie is among them and closes the window.
fn capture_session<R: tauri::Runtime>(
    webview: &WebviewWindow<R>,
    provider: &str,
    site: &Url,
) -> Result<(), String> {
    let host = site.host_str().unwrap_or_default();
    let cookies = webview.cookies().map_err(to_ipc)?;
    let header = cookie_header(
        cookies
            .iter()
            .filter(|c| cookie_matches_host(c.domain(), host))
            .map(|c| (c.name(), c.value())),
    );
    if !header.contains(&format!("{CLEARANCE_COOKIE}=")) {
        return Ok(());
    }
    let lib = open_library()?;
    let mut settings = settings_of(&lib)?;
    settings.provider_sessions.insert(
        provider.to_string(),
        ProviderSession {
            cookies: header,
            user_agent: BROWSER_USER_AGENT.to_string(),
            saved_at: Some(chrono::Utc::now().to_rfc3339()),
        },
    );
    library::set_settings(&lib, &settings).map_err(to_ipc)?;
    log::info!("{provider}: browser session saved");
    webview.close().map_err(to_ipc)
}

/// The Connect window may only navigate within the provider's site and Cloudflare's challenge
/// pages, over HTTPS; anything else (ads, outbound links, plaintext) is blocked.
pub fn allowed_navigation(url: &Url, site: &Url) -> bool {
    if url.scheme() == "about" {
        return true;
    }
    let host = url.host_str().unwrap_or_default();
    let site_host = site.host_str().unwrap_or_default();
    url.scheme() == "https"
        && (host == site_host
            || host.ends_with(&format!(".{site_host}"))
            || host == "challenges.cloudflare.com")
}

/// Whether a cookie set for `domain` is sent to `host`. Cookies the site scoped to a parent
/// domain (`Domain=.mangadot.net`, as Cloudflare does) must count too: macOS reports them with
/// the leading dot, which Tauri's own per-URL filter compares as an exact string and drops.
/// A cookie without a domain cannot be attributed to the site and is left out.
pub fn cookie_matches_host(domain: Option<&str>, host: &str) -> bool {
    let Some(domain) = domain.map(|d| d.trim_start_matches('.')) else {
        return false;
    };
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// `name=value; name2=value2`, as the HTTP `Cookie` header wants it.
pub fn cookie_header<'a>(cookies: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    cookies
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::cookie_matches_host;

    #[test]
    fn parent_domain_cookies_apply_to_the_site_host() {
        assert!(cookie_matches_host(Some(".mangadot.net"), "mangadot.net"));
        assert!(cookie_matches_host(Some("mangadot.net"), "mangadot.net"));
        assert!(cookie_matches_host(
            Some(".mangadot.net"),
            "www.mangadot.net"
        ));
    }

    #[test]
    fn other_hosts_are_left_out() {
        assert!(!cookie_matches_host(Some("cloudflare.com"), "mangadot.net"));
        assert!(!cookie_matches_host(
            Some(".notmangadot.net"),
            "mangadot.net"
        ));
    }
}
