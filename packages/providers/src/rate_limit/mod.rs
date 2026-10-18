//! Request pacing: a [`TokenBucket`] per host, shared process-wide through [`HostLimiters`] so
//! every client and call to the same host draws from one budget. With
//! [`HostLimiters::share_across_processes`] the buckets live in files, so the CLI and the
//! desktop app share that budget too.

mod bucket;
mod pool;
mod shared;

pub use bucket::TokenBucket;
pub use pool::{host_key, HostLimiters};
