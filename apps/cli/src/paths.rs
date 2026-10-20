use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;
use mangas_core::library;
use mangas_core::storage::Folders;

fn dirs() -> anyhow::Result<ProjectDirs> {
    ProjectDirs::from("dev", "kerbaras", "mangas").context("no home directory")
}

/// `~/.local/share/mangas/mangas.db` (platform equivalent elsewhere). Always the platform dir:
/// it is where the data-folder override itself is stored.
pub fn library_db() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.data_dir().join("mangas.db"))
}

/// Per-host rate limit buckets shared with the desktop app (platform dir, like the database).
pub fn rate_limits() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.data_dir().join("rate-limits"))
}

/// Folder chosen in the desktop app's Settings, so the CLI and the app see the same chapters
/// and models; the platform data dir when none is set or the library cannot be read.
pub fn data_dir() -> anyhow::Result<PathBuf> {
    let custom = library::open(&library_db()?)
        .and_then(|lib| library::get_settings(&lib))
        .ok()
        .and_then(|s| s.data_dir.filter(|d| !d.is_empty()));
    match custom {
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

/// Every folder the app writes to, for storage usage and maintenance.
pub fn folders() -> anyhow::Result<Folders> {
    let app = dirs()?.data_dir().to_path_buf();
    Ok(Folders {
        data_dir: data_dir()?,
        database: library_db()?,
        covers: mangas_core::covers::dir(&app),
        logs: mangas_core::logging::log_dir(&app),
    })
}
