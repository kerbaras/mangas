//! WebView2: a `NavigationCompleted` with `IsSuccess == false` is the Connect window's failed
//! load (WebView2 raises it for the top-level document only, after showing its error page).

use tauri::{Runtime, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2, ICoreWebView2Controller, ICoreWebView2NavigationCompletedEventArgs,
    COREWEBVIEW2_WEB_ERROR_STATUS,
};
use webview2_com::{take_pwstr, NavigationCompletedEventHandler};
use windows_core::{BOOL, PWSTR};

use super::failure::{reason, webview2};

pub fn watch_loads<R: Runtime>(window: &WebviewWindow<R>, provider: &'static str) {
    let target = window.clone();
    let hooked = window.with_webview(move |platform| {
        if let Err(e) = attach(&platform.controller(), target, provider) {
            log::warn!("{provider}: connect window load errors not watched: {e}");
        }
    });
    if let Err(e) = hooked {
        log::warn!("{provider}: connect window load errors not watched: {e}");
    }
}

fn attach<R: Runtime>(
    controller: &ICoreWebView2Controller,
    window: WebviewWindow<R>,
    provider: &'static str,
) -> windows_core::Result<()> {
    // SAFETY: COM calls on the live controller, on the main thread (`with_webview`).
    let webview = unsafe { controller.CoreWebView2()? };
    let handler = NavigationCompletedEventHandler::create(Box::new(move |view, args| {
        let (Some(view), Some(args)) = (view, args) else {
            return Ok(());
        };
        if let Some(reason) = failed(&view, &args)? {
            super::fail(&window, provider, reason);
        }
        Ok(())
    }));
    let mut token = 0i64;
    // SAFETY: as above; the handler lives as long as the webview.
    unsafe { webview.add_NavigationCompleted(&handler, &mut token) }
}

fn failed(
    view: &ICoreWebView2,
    args: &ICoreWebView2NavigationCompletedEventArgs,
) -> windows_core::Result<Option<String>> {
    let mut ok = BOOL::default();
    let mut status = COREWEBVIEW2_WEB_ERROR_STATUS::default();
    let mut uri = PWSTR::null();
    // SAFETY: out-parameters of COM getters on live objects; `take_pwstr` frees the string.
    unsafe {
        args.IsSuccess(&mut ok)?;
        if ok.as_bool() {
            return Ok(None);
        }
        args.WebErrorStatus(&mut status)?;
        view.Source(&mut uri)?;
    }
    Ok(reason(&take_pwstr(uri), webview2(status.0)))
}
