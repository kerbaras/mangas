//! An export reads chapter files out of the data folder for its whole run. It holds the data
//! folder lease (`models/.detectors.lease`, the one a move takes exclusively) shared until it
//! ends: a move is refused while an export reads, and an export is refused while a move runs,
//! in this process or another.

use std::path::Path;

use crate::domain::Result;
use crate::ml::lease::{self, Lease};

/// The lease an export holds from before it looks up its chapter folders until it ends.
pub fn hold_for_export(models_dir: &Path) -> Result<Lease> {
    lease::share(models_dir)
}
