//! Watches a provider's Connect window so the UI stops waiting at once: closing the window ends
//! the wait (the UI checks for a saved session), and a page that fails to load (TLS, DNS)
//! reports why and closes the window instead of leaving WebKit's error page up.

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
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
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

#[cfg(target_os = "linux")]
mod linux {
    use mangas_core::domain::{system_has_root_certificates, webview_tls_failure, CertProblem};
    use tauri::{Runtime, WebviewWindow};
    use webkit2gtk::gio::TlsCertificateFlags;
    use webkit2gtk::{glib, NetworkError, PolicyError, WebViewExt};

    /// WebKitGTK's `load-failed` signals, for the main frame of the Connect window.
    pub fn watch_loads<R: Runtime>(window: &WebviewWindow<R>, provider: &'static str) {
        let target = window.clone();
        let hooked = window.with_webview(move |platform| {
            let view = platform.inner();
            let tls = target.clone();
            view.connect_load_failed_with_tls_errors(move |_, uri, _, flags| {
                let host = url::Url::parse(uri)
                    .ok()
                    .and_then(|u| u.host_str().map(str::to_string))
                    .unwrap_or_else(|| uri.to_string());
                let reason =
                    webview_tls_failure(&host, &problem(flags), system_has_root_certificates());
                super::fail(&tls, provider, reason);
                true
            });
            view.connect_load_failed(move |_, _, uri, error| {
                if ignorable(error) {
                    return false;
                }
                let reason = format!("network: couldn't load {uri}: {}", error.message());
                super::fail(&target, provider, reason);
                true
            });
        });
        if let Err(e) = hooked {
            log::warn!("{provider}: connect window load errors not watched: {e}");
        }
    }

    /// WebKitGTK's certificate flags in the words the HTTP client uses for the same problem.
    fn problem(flags: TlsCertificateFlags) -> CertProblem {
        let has = |flag| flags.contains(flag);
        if has(TlsCertificateFlags::BAD_IDENTITY) {
            CertProblem::WrongHost
        } else if has(TlsCertificateFlags::EXPIRED) {
            CertProblem::Expired
        } else if has(TlsCertificateFlags::NOT_ACTIVATED) {
            CertProblem::NotYetValid
        } else if has(TlsCertificateFlags::REVOKED) {
            CertProblem::Revoked
        } else if has(TlsCertificateFlags::UNKNOWN_CA) {
            CertProblem::UnknownIssuer
        } else {
            CertProblem::Other(format!("{flags:?}"))
        }
    }

    /// A load the app stopped itself: a blocked navigation or a page replaced by the next one.
    fn ignorable(error: &glib::Error) -> bool {
        error.matches(NetworkError::Cancelled)
            || error.matches(PolicyError::FrameLoadInterruptedByPolicyChange)
    }
}
