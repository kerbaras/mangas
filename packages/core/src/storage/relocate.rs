use std::path::{Path, PathBuf};

use super::layout::DATA_SUBDIRS;
use crate::domain::{Error, Result};

/// Refuses a move that would copy a folder into itself or onto the current data folder.
/// Both paths should be absolute and resolved.
pub fn check_target(from: &Path, to: &Path) -> Result<()> {
    if from == to {
        return Err(Error::Other("that's already the data folder".into()));
    }
    for sub in DATA_SUBDIRS {
        let (old, new) = (from.join(sub), to.join(sub));
        if old.starts_with(&new) || new.starts_with(&old) {
            return Err(Error::Other(format!(
                "the new data folder can't be inside the current {sub} folder or contain it"
            )));
        }
    }
    Ok(())
}

/// `path` re-rooted from `from` to `to`; `None` when it isn't under `from`.
pub fn rebase(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    path.strip_prefix(from).ok().map(|rest| to.join(rest))
}

#[cfg(test)]
mod tests {
    use super::{check_target, rebase};
    use std::path::{Path, PathBuf};

    #[test]
    fn rejects_same_and_nested_targets() {
        let from = Path::new("/data");
        assert!(check_target(from, Path::new("/data")).is_err());
        assert!(check_target(from, Path::new("/data/chapters/x")).is_err());
        assert!(check_target(Path::new("/new/chapters/old"), Path::new("/new")).is_err());
        assert!(check_target(from, Path::new("/data/new")).is_ok());
        assert!(check_target(from, Path::new("/mnt/big")).is_ok());
    }

    #[test]
    fn rebases_paths_under_the_old_root_only() {
        let (from, to) = (Path::new("/data"), Path::new("/mnt/big"));
        assert_eq!(
            rebase(Path::new("/data/chapters/a/1"), from, to),
            Some(PathBuf::from("/mnt/big/chapters/a/1"))
        );
        assert_eq!(rebase(Path::new("/elsewhere/a/1"), from, to), None);
        assert_eq!(rebase(Path::new("/database/x"), from, to), None);
    }
}
