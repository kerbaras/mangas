//! Exit codes beyond anyhow's 1.

use std::fmt;

/// A failure that ends the process with its own exit `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exit {
    pub code: u8,
    pub message: String,
}

impl fmt::Display for Exit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Exit {}

/// The exit code for a failed command: an [`Exit`]'s own, 1 for anything else.
pub fn exit_code(error: &anyhow::Error) -> u8 {
    error.downcast_ref::<Exit>().map_or(1, |e| e.code)
}
