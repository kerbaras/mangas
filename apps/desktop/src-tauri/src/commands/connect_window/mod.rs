//! Watches a provider's Connect window so the UI stops waiting at once: closing the window ends
//! the wait (the UI checks for a saved session), and a page that fails to load (TLS, DNS)
//! reports why and closes the window instead of leaving the webview's error page up. Each
//! platform hooks its webview's own load-failed event: WebKitGTK `load-failed`, WebView2
//! `NavigationCompleted`, WKWebView `didFail(Provisional)Navigation`.

mod failure;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use serde::Serialize;
use tauri::{Emitter, Manager, Runtime, WebviewWindow, WindowEvent};

pub const CONNECT_EVENT: &str = "provider-connect";

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ConnectEnd {
    Closed { provider: String },
    Failed { provider: String, reason: String },
}

pub fn watch<R: Runtime>(window: &WebviewWindow<R>, provider: &'static str) {
    let app = window.app_handle().clone();
    #[cfg(target_os = "macos")]
    let label = window.label().to_string();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            #[cfg(target_os = "macos")]
            macos::forget(&label);
            emit(
                &app,
                ConnectEnd::Closed {
                    provider: provider.to_string(),
                },
            );
        }
    });
    #[cfg(target_os = "linux")]
    linux::watch_loads(window, provider);
    #[cfg(target_os = "macos")]
    macos::watch_loads(window, provider);
    #[cfg(windows)]
    windows::watch_loads(window, provider);
}

fn emit<R: Runtime>(app: &tauri::AppHandle<R>, end: ConnectEnd) {
    if let Err(e) = app.emit(CONNECT_EVENT, end) {
        log::warn!("connect window event not sent: {e}");
    }
}

/// Reports a failed load, then closes the window (which also sends `Closed`, after this).
fn fail<R: Runtime>(window: &WebviewWindow<R>, provider: &'static str, reason: String) {
    log::warn!("{provider}: connect window failed to load: {reason}");
    let provider = provider.to_string();
    emit(window.app_handle(), ConnectEnd::Failed { provider, reason });
    if let Err(e) = window.close() {
        log::warn!("connect window not closed: {e}");
    }
}
