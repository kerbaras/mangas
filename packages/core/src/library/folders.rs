//! Which folders the desktop may open in the file manager: only real directories inside the
//! app's own folders or a folder it exported to, so the webview can't launch arbitrary files.

use std::path::{Path, PathBuf};

use super::{list_exports, Library};
use crate::domain::IoAt;
use crate::domain::{Error, Result};

/// How many recent exports count as export folders.
const RECENT_EXPORTS: usize = 200;

/// `path` resolved (symlinks, `..`), if it is a directory inside one of `roots` or exactly one
/// of `exact` (export folders: exporting to `~` must not open every folder under it).
pub fn openable_folder(path: &Path, roots: &[PathBuf], exact: &[PathBuf]) -> Result<PathBuf> {
    let dir = path.canonicalize().at(path)?;
    if !dir.is_dir() {
        return Err(Error::Other(format!("{} is not a folder", path.display())));
    }
    let resolved = |dirs: &[PathBuf]| -> Vec<PathBuf> {
        dirs.iter().filter_map(|d| d.canonicalize().ok()).collect()
    };
    let inside =
        resolved(roots).iter().any(|root| dir.starts_with(root)) || resolved(exact).contains(&dir);
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
