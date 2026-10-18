//! The last runtime download or load failure in this process, per models folder, shown by the
//! status.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

static FAILURE: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);

pub fn record(models_dir: &Path, reason: String) {
    if let Ok(mut failure) = FAILURE.lock() {
        *failure = Some((models_dir.to_path_buf(), reason));
    }
}

pub fn clear() {
    if let Ok(mut failure) = FAILURE.lock() {
        *failure = None;
    }
}

/// The failure recorded for `models_dir`; another folder's failure doesn't apply here.
pub fn failure(models_dir: &Path) -> Option<String> {
    let failure = FAILURE.lock().ok()?;
    failure
        .as_ref()
        .filter(|(dir, _)| dir == models_dir)
        .map(|(_, reason)| reason.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_applies_only_to_its_own_folder() {
        record(Path::new("/a/models"), "bad archive".into());
        assert_eq!(
            failure(Path::new("/a/models")).as_deref(),
            Some("bad archive")
        );
        assert_eq!(failure(Path::new("/b/models")), None);
        clear();
        assert_eq!(failure(Path::new("/a/models")), None);
    }
}
