//! Which folders the desktop may open in the file manager: only real directories inside the
//! app's own folders or a folder it exported to, so the webview can't launch arbitrary files.

use std::path::{Path, PathBuf};

use super::{list_exports, Library};
use crate::domain::{Error, Result};

/// How many recent exports count as export folders.
const RECENT_EXPORTS: usize = 200;

/// `path` resolved (symlinks, `..`), if it is a directory inside one of `roots`.
pub fn openable_folder(path: &Path, roots: &[PathBuf]) -> Result<PathBuf> {
    let dir = path.canonicalize()?;
    if !dir.is_dir() {
        return Err(Error::Other(format!("{} is not a folder", path.display())));
    }
    let inside = roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|root| dir.starts_with(root));
    if !inside {
        return Err(Error::Other(format!(
            "{} is outside the app's folders",
            path.display()
        )));
    }
    Ok(dir)
}

/// Folders the app exported files to, newest exports first, without duplicates.
pub fn export_folders(lib: &Library) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for record in list_exports(lib, RECENT_EXPORTS)? {
        let parent = Path::new(&record.file).parent().map(Path::to_path_buf);
        match parent {
            Some(dir) if record.exported_at.is_some() && !dirs.contains(&dir) => dirs.push(dir),
            _ => {}
        }
    }
    Ok(dirs)
}
