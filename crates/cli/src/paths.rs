use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;

fn dirs() -> anyhow::Result<ProjectDirs> {
    ProjectDirs::from("dev", "kerbaras", "mangas").context("no home directory")
}

/// `~/.local/share/mangas/mangas.db` (platform equivalent elsewhere).
pub fn library_db() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.data_dir().join("mangas.db"))
}

/// Root directory for downloaded chapters.
pub fn downloads() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.data_dir().join("chapters"))
}

/// Cache directory for downloaded ONNX detector models.
pub fn models() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.data_dir().join("models"))
}
