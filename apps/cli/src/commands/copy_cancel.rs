//! Ctrl-C during a USB copy cancels it (the core removes the partial book and keeps the one
//! already there); outside a copy it ends the CLI as before.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static COPYING: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);
static LISTENING: OnceLock<()> = OnceLock::new();

/// Runs `copy` with Ctrl-C bound to its cancel flag (`cancelled` in the core copy).
pub fn cancellable<T>(copy: impl FnOnce(&(dyn Fn() -> bool + Sync)) -> T) -> T {
    listen();
    CANCELLED.store(false, Ordering::SeqCst);
    COPYING.store(true, Ordering::SeqCst);
    let out = tokio::task::block_in_place(|| copy(&|| CANCELLED.load(Ordering::SeqCst)));
    COPYING.store(false, Ordering::SeqCst);
    out
}

fn listen() {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    LISTENING.get_or_init(|| {
        runtime.spawn(async {
            while tokio::signal::ctrl_c().await.is_ok() {
                if !COPYING.load(Ordering::SeqCst) {
                    std::process::exit(130);
                }
                CANCELLED.store(true, Ordering::SeqCst);
            }
        });
    });
}
