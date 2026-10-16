//! Provider list and the browser-session bridge for sites behind a verification wall: the
//! site opens in its own window, the user passes the check, and the resulting cookies are
//! stored with the user agent that obtained them for the Rust HTTP client to reuse.

use mangas_core::library::{self, ProviderSession};
use mangas_providers::{ProviderInfo, PROVIDERS};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use super::{open_library, settings_of, to_ipc};

/// A current desktop Chrome user agent: Cloudflare binds `cf_clearance` to it, so the same
/// string goes out with every API request that carries the cookie.
pub const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36";
const CLEARANCE_COOKIE: &str = "cf_clearance";

#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    PROVIDERS.to_vec()
}

/// Opens `provider`'s site in a window. Once the verification cookie appears the session is
/// saved and the window closes itself.
#[tauri::command]
pub fn connect_provider(app: AppHandle, provider: String) -> Result<(), String> {
    let info = mangas_providers::info(&provider)
        .ok_or_else(|| format!("unknown provider '{provider}'"))?;
    let site = Url::parse(info.site).map_err(to_ipc)?;
    let label = format!("connect-{}", info.id);
    if let Some(existing) = app.get_webview_window(&label) {
        return existing.set_focus().map_err(to_ipc);
    }
    let id = info.id;
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(site.clone()))
        .title(format!("Connect {}", info.name))
        .inner_size(1000.0, 760.0)
        .user_agent(BROWSER_USER_AGENT)
        .on_page_load(move |webview, _| {
            if let Err(e) = capture_session(&webview, id, &site) {
                log::warn!("{id}: could not read session cookies: {e}");
            }
        })
        .build()
        .map_err(to_ipc)?;
    Ok(())
}

#[tauri::command]
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
    let cookies = webview.cookies_for_url(site.clone()).map_err(to_ipc)?;
    let header = cookie_header(cookies.iter().map(|c| (c.name(), c.value())));
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

/// `name=value; name2=value2`, as the HTTP `Cookie` header wants it.
pub fn cookie_header<'a>(cookies: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    cookies
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}
