//! WKWebView: wry's navigation delegate has no `webView:didFailProvisionalNavigation:withError:`
//! (where a TLS or DNS failure lands), so the Connect window adds that method to the delegate's
//! class and sets the delegate again, because WebKit reads which optional methods a delegate
//! has when it is set.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use objc2::ffi;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Imp, Sel};
use objc2::sel;
use objc2_foundation::{NSError, NSNumber, NSString};
use objc2_web_kit::WKWebView;
use tauri::{Runtime, WebviewWindow};

use super::failure::{reason, wkwebview};

struct Watch {
    label: String,
    uri: String,
    fail: Box<dyn FnOnce(String) + Send>,
}

/// Watched webviews by address; an entry leaves on the first failure or when its window closes.
fn watched() -> &'static Mutex<HashMap<usize, Watch>> {
    static WATCHED: OnceLock<Mutex<HashMap<usize, Watch>>> = OnceLock::new();
    WATCHED.get_or_init(Default::default)
}

pub fn watch_loads<R: Runtime>(window: &WebviewWindow<R>, provider: &'static str) {
    let target = window.clone();
    let uri = window.url().map(|u| u.to_string()).unwrap_or_default();
    let hooked = window.with_webview(move |platform| {
        let view = platform.inner() as *const WKWebView;
        let watch = Watch {
            label: target.label().to_string(),
            uri,
            fail: Box::new(move |reason| super::fail(&target, provider, reason)),
        };
        if let Ok(mut map) = watched().lock() {
            map.insert(view as usize, watch);
        }
        // SAFETY: `inner()` is this window's live WKWebView; `with_webview` runs on the main thread.
        unsafe { hook(&*view) };
    });
    if let Err(e) = hooked {
        log::warn!("{provider}: connect window load errors not watched: {e}");
    }
}

/// Drops the watch of a closed window.
pub fn forget(label: &str) {
    if let Ok(mut map) = watched().lock() {
        map.retain(|_, w| w.label != label);
    }
}

unsafe fn hook(view: &WKWebView) {
    let Some(delegate) = view.navigationDelegate() else {
        log::warn!("connect window has no navigation delegate; load errors not watched");
        return;
    };
    let class = ffi::object_getClass(Retained::as_ptr(&delegate).cast());
    let sel = sel!(webView:didFailProvisionalNavigation:withError:);
    if !ffi::class_respondsToSelector(class, sel).as_bool() {
        let method: DidFail = did_fail;
        let imp = std::mem::transmute::<DidFail, Imp>(method);
        ffi::class_addMethod(class.cast_mut(), sel, imp, c"v@:@@@".as_ptr());
    }
    view.setNavigationDelegate(Some(&delegate));
}

type DidFail = unsafe extern "C-unwind" fn(&AnyObject, Sel, &WKWebView, *const AnyObject, &NSError);

/// The Secure Transport status WebKit puts in `userInfo` for a refused certificate, if any.
fn stream_status(error: &NSError) -> Option<isize> {
    let key = NSString::from_str("_kCFStreamErrorCodeKey");
    let value = error.userInfo().objectForKey(&key)?;
    value.downcast::<NSNumber>().ok().map(|n| n.integerValue())
}

unsafe extern "C-unwind" fn did_fail(
    _delegate: &AnyObject,
    _cmd: Sel,
    view: &WKWebView,
    _navigation: *const AnyObject,
    error: &NSError,
) {
    let load = wkwebview(
        &error.domain().to_string(),
        error.code(),
        stream_status(error),
        &error.localizedDescription().to_string(),
    );
    let key = view as *const WKWebView as usize;
    let uri = match watched().lock() {
        Ok(map) => map.get(&key).map(|w| w.uri.clone()),
        Err(_) => None,
    };
    let Some(reason) = uri.and_then(|uri| reason(&uri, load)) else {
        return;
    };
    // Taken out before the call: `fail` closes the window, whose `forget` locks the map again.
    let watch = watched().lock().ok().and_then(|mut map| map.remove(&key));
    if let Some(watch) = watch {
        (watch.fail)(reason);
    }
}
