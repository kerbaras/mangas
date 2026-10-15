use std::path::PathBuf;

use directories::ProjectDirs;

pub fn library_db() -> Result<PathBuf, String> {
    ProjectDirs::from("dev", "kerbaras", "mangas")
        .map(|d| d.data_dir().join("mangas.db"))
        .ok_or_else(|| "no home directory".to_string())
}
