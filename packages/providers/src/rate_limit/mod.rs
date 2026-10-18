//! Request pacing: a [`TokenBucket`] per host, shared process-wide through [`HostLimiters`] so
//! every client and call to the same host draws from one budget.

mod bucket;
mod pool;

pub use bucket::TokenBucket;
pub use pool::{host_key, HostLimiters};
