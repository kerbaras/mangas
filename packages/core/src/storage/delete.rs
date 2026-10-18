use std::fs;
use std::path::Path;

use serde::Serialize;

use super::tree::dir_size;
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

/// Whether `path` looks like a folder the downloader made for this manga:
/// `…/<source>/<manga id>/<chapter>`. Anything else (a path edited by hand, a moved library)
/// is never deleted.
pub fn in_download_layout(path: &Path, source: &str, manga_id: &str) -> bool {
    let parent = path.parent();
    path.is_absolute()
        && path.file_name().is_some()
        && file_name(parent) == Some(safe_component(manga_id))
        && file_name(parent.and_then(Path::parent)) == Some(safe_component(source))
}

fn file_name(path: Option<&Path>) -> Option<String> {
    path.and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
}

/// Deletes the downloaded chapter folders of one followed manga and forgets them; the manga,
/// its chapter list and metadata stay so chapters can be downloaded again.
pub fn delete_downloads(lib: &Library, source: &str, manga_id: &str) -> Result<Deleted> {
    if is_local(source) {
        return Err(Error::Other(
            "imported titles have no source to download them again from; remove the title instead"
                .into(),
        ));
    }
    let mut deleted = Deleted::default();
    let mut manga_dir = None;
    for chapter in downloaded_chapters(lib, Some((source, manga_id)))? {
        let path = Path::new(&chapter.path);
        if !in_download_layout(path, source, manga_id) {
            deleted.kept.push(chapter.path.clone());
            continue;
        }
        if path.exists() {
            deleted.bytes += dir_size(path);
            fs::remove_dir_all(path)?;
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
        let ok = Path::new("/data/chapters/mangadex/abc/ch-1");
        assert!(in_download_layout(ok, "mangadex", "abc"));
        assert!(!in_download_layout(ok, "mangadex", "other"));
        assert!(!in_download_layout(ok, "tapas", "abc"));
        assert!(!in_download_layout(Path::new("/"), "mangadex", "abc"));
        assert!(!in_download_layout(
            Path::new("chapters/mangadex/abc/ch-1"),
            "mangadex",
            "abc"
        ));
    }
}
