use std::path::PathBuf;

use directories::ProjectDirs;

fn data_dir() -> Result<PathBuf, String> {
    ProjectDirs::from("dev", "kerbaras", "mangas")
        .map(|d| d.data_dir().to_path_buf())
        .ok_or_else(|| "no home directory".to_string())
}

pub fn library_db() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("mangas.db"))
}

/// Root directory for downloaded chapters (same layout as the CLI).
pub fn downloads() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("chapters"))
}
