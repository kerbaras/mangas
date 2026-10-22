use std::fs;
use std::path::Path;

use serde::Serialize;

use super::siblings::chapter_files;
use super::tree::dir_size;
use crate::domain::IoAt;
use crate::domain::{Error, Result};
use crate::download::safe_component;
use crate::library::{downloaded_chapters, forget_download, is_local, Library};

/// What deleting a title's downloads freed, and the folders it left alone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Deleted {
    pub chapters: usize,
    pub bytes: u64,
    /// Recorded folders outside the download layout; left on disk and still recorded.
    pub kept: Vec<String>,
}

/// Whether `path` is a folder the downloader made for this manga:
/// `<root>/<source>/<manga id>/<chapter>`, with `root` the chapters folder. Anything else (a
/// path edited by hand, a folder outside the data folder) is never deleted.
pub fn in_download_layout(path: &Path, root: &Path, source: &str, manga_id: &str) -> bool {
    let manga = path.parent();
    let src = manga.and_then(Path::parent);
    path.file_name().is_some()
        && file_name(manga) == Some(safe_component(manga_id))
        && file_name(src) == Some(safe_component(source))
        && src.and_then(Path::parent) == Some(root)
}

fn file_name(path: Option<&Path>) -> Option<String> {
    path.and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
}

/// Deletes the downloaded chapter folders (and `.cbz` archives) of one followed manga under
/// `chapters_root` and forgets them; the manga, its chapter list and metadata stay so chapters
/// can be downloaded again. Links and plain files on record are kept, never followed.
pub fn delete_downloads(
    lib: &Library,
    chapters_root: &Path,
    source: &str,
    manga_id: &str,
) -> Result<Deleted> {
    if is_local(source) {
        return Err(Error::Other(
            "imported titles have no source to download them again from; remove the title instead"
                .into(),
        ));
    }
    let mut deleted = Deleted::default();
    let root = fs::canonicalize(chapters_root).unwrap_or_else(|_| chapters_root.to_path_buf());
    let mut manga_dir = None;
    for chapter in downloaded_chapters(lib, Some((source, manga_id)))? {
        let recorded = Path::new(&chapter.path);
        let path = fs::canonicalize(recorded).unwrap_or_else(|_| recorded.to_path_buf());
        let link = fs::symlink_metadata(recorded).is_ok_and(|m| m.is_symlink());
        let file = path.exists() && !path.is_dir();
        if link || file || !in_download_layout(&path, &root, source, manga_id) {
            deleted.kept.push(chapter.path.clone());
            continue;
        }
        if path.is_dir() {
            deleted.bytes += dir_size(&path);
            fs::remove_dir_all(&path).at(&path)?;
        }
        for (file, bytes) in chapter_files(&path) {
            deleted.bytes += bytes;
            fs::remove_file(&file).at(&file)?;
        }
        forget_download(lib, source, &chapter.chapter_id)?;
        deleted.chapters += 1;
        manga_dir = path.parent().map(Path::to_path_buf);
    }
    if let Some(dir) = manga_dir {
        fs::remove_dir(dir).ok();
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::in_download_layout;
    use std::path::Path;

    #[test]
    fn only_folders_under_source_and_manga_count_as_downloads() {
        let root = Path::new("/data/chapters");
        let ok = Path::new("/data/chapters/mangadex/abc/ch-1");
        assert!(in_download_layout(ok, root, "mangadex", "abc"));
        assert!(!in_download_layout(ok, root, "mangadex", "other"));
        assert!(!in_download_layout(ok, root, "tapas", "abc"));
        assert!(!in_download_layout(Path::new("/"), root, "mangadex", "abc"));
        let elsewhere = Path::new("/home/me/mangadex/abc/ch-1");
        assert!(!in_download_layout(elsewhere, root, "mangadex", "abc"));
    }
}
