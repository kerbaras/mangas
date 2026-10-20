//! The worker threads that decode and encode pages side by side. A pool of its own, as wide as
//! the pages a run keeps in flight, so a many-core machine doesn't hold a page (and a malloc
//! arena) per core.

use std::sync::OnceLock;

use rayon::{ThreadPool, ThreadPoolBuilder};

/// Pages decoded and encoded at once. Each holds up to ~100 MB at its peak on a 2722x3870 scan;
/// three keep the export within ~150 MB of one page at a time and still about 3 times as fast.
pub(super) const WORKERS: usize = 3;

fn pool() -> Option<&'static ThreadPool> {
    static POOL: OnceLock<Option<ThreadPool>> = OnceLock::new();
    POOL.get_or_init(|| {
        ThreadPoolBuilder::new()
            .num_threads(WORKERS)
            .thread_name(|i| format!("export-page-{i}"))
            .build()
            .map_err(|e| log::warn!("export page pool: {e}; using the shared one"))
            .ok()
    })
    .as_ref()
}

/// Runs `f` (its parallel iterators) on the page workers.
pub(super) fn install<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    match pool() {
        Some(p) => p.install(f),
        None => f(),
    }
}
