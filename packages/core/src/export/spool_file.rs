//! The spool's file in the export folder. It is unlinked as soon as it is created, so the OS
//! frees it however the export ends (done, failed, cancelled, killed). For that instant it is
//! named `.mangas-spool-*.tmp`, so one left by a run killed right then says what it is. Nothing
//! removes such files: another export into the same folder (CLI and app at once) may own one.

use std::fs::File;
use std::path::Path;

const PREFIX: &str = ".mangas-spool-";
const SUFFIX: &str = ".tmp";

/// Creates the spool file in `dir` (created if missing), already unlinked.
pub(super) fn create_in(dir: &Path) -> std::io::Result<File> {
    std::fs::create_dir_all(dir)?;
    let named = tempfile::Builder::new()
        .prefix(PREFIX)
        .suffix(SUFFIX)
        .tempfile_in(dir)?;
    Ok(named.into_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn the_spool_has_no_name_in_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let _spool = create_in(&dir.path().join("out")).unwrap();
        assert!(names(&dir.path().join("out")).is_empty());
    }

    #[test]
    fn other_spools_in_the_folder_are_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let old = File::create(dir.path().join(".mangas-spool-other.tmp")).unwrap();
        old.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
            .unwrap();
        let _spool = create_in(dir.path()).unwrap();
        assert_eq!(names(dir.path()), [".mangas-spool-other.tmp"]);
    }
}
