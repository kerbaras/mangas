//! WebKitGTK: the `load-failed` signals of the Connect window's main frame.

use mangas_core::domain::CertProblem;
use tauri::{Runtime, WebviewWindow};
use webkit2gtk::gio::TlsCertificateFlags;
use webkit2gtk::{glib, NetworkError, PolicyError, WebViewExt};

use super::failure::{reason, Load};

pub fn watch_loads<R: Runtime>(window: &WebviewWindow<R>, provider: &'static str) {
    let target = window.clone();
    let hooked = window.with_webview(move |platform| {
        let view = platform.inner();
        let tls = target.clone();
        view.connect_load_failed_with_tls_errors(move |_, uri, _, flags| {
            if let Some(reason) = reason(uri, Load::Certificate(problem(flags))) {
                super::fail(&tls, provider, reason);
            }
            true
        });
        view.connect_load_failed(move |_, _, uri, error| match reason(uri, load(error)) {
            Some(reason) => {
                super::fail(&target, provider, reason);
                true
            }
            None => false,
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

fn load(error: &glib::Error) -> Load {
    if error.matches(NetworkError::Cancelled)
        || error.matches(PolicyError::FrameLoadInterruptedByPolicyChange)
    {
        Load::Ignored
    } else {
        Load::Network(error.message().to_string())
    }
}
