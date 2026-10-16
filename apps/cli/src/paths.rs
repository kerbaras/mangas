use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;
use mangas_core::library;

fn dirs() -> anyhow::Result<ProjectDirs> {
    ProjectDirs::from("dev", "kerbaras", "mangas").context("no home directory")
}

/// `~/.local/share/mangas/mangas.db` (platform equivalent elsewhere). Always the platform dir:
/// it is where the data-folder override itself is stored.
pub fn library_db() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.data_dir().join("mangas.db"))
}

/// Folder chosen in the desktop app's Settings, or the platform data dir, so the CLI and the
/// app see the same chapters and models.
fn data_dir() -> anyhow::Result<PathBuf> {
    let lib = library::open(&library_db()?)?;
    let settings = library::get_settings(&lib)?;
    match settings.data_dir.filter(|s| !s.is_empty()) {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Ok(dirs()?.data_dir().to_path_buf()),
    }
}

/// Root directory for downloaded chapters.
pub fn downloads() -> anyhow::Result<PathBuf> {
    Ok(data_dir()?.join("chapters"))
}

/// Cache directory for downloaded ONNX detector models.
pub fn models() -> anyhow::Result<PathBuf> {
    Ok(data_dir()?.join("models"))
}
