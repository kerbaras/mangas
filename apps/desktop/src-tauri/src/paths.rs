use std::path::PathBuf;

use directories::ProjectDirs;
use mangas_core::library::Settings;

/// Platform data dir: home of the library database and the log files.
pub fn default_data_dir() -> Result<PathBuf, String> {
    ProjectDirs::from("dev", "kerbaras", "mangas")
        .map(|d| d.data_dir().to_path_buf())
        .ok_or_else(|| "no home directory".to_string())
}

/// The library database always lives in the platform data dir: it is where the
/// `data_dir` override itself is stored.
pub fn library_db() -> Result<PathBuf, String> {
    Ok(default_data_dir()?.join("mangas.db"))
}

/// Per-host rate limit buckets shared with the CLI (platform dir, like the database).
pub fn rate_limits() -> Result<PathBuf, String> {
    Ok(default_data_dir()?.join("rate-limits"))
}

/// Root chosen by the user in Settings, or the platform data dir.
pub fn data_dir(settings: &Settings) -> Result<PathBuf, String> {
    match settings.data_dir.as_deref().filter(|s| !s.is_empty()) {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => default_data_dir(),
    }
}

/// Root directory for downloaded chapters (same layout as the CLI).
pub fn downloads(settings: &Settings) -> Result<PathBuf, String> {
    Ok(data_dir(settings)?.join("chapters"))
}

/// Cache directory for downloaded ONNX detector models and the ONNX Runtime library.
pub fn models(settings: &Settings) -> Result<PathBuf, String> {
    Ok(data_dir(settings)?.join("models"))
}

/// Default folder for exported books: `kindle` in the data folder.
pub fn exports(settings: &Settings) -> Result<PathBuf, String> {
    Ok(data_dir(settings)?.join("kindle"))
}

/// Cover cache: next to the library database, whatever `data_dir` says.
pub fn covers() -> Result<PathBuf, String> {
    Ok(default_data_dir()?.join("covers"))
}

/// The profile when neither the request nor the stored settings name one (same as the CLI).
pub const DEFAULT_PROFILE: &str = "kindle-paperwhite12";

/// Smart-cut preview pages, emptied at start-up; served to the webview through the asset protocol.
pub fn cut_previews() -> Result<PathBuf, String> {
    Ok(default_data_dir()?.join("cut-preview"))
}

/// Log files live next to the library database, whatever `data_dir` says.
pub fn log_dir() -> Result<PathBuf, String> {
    Ok(mangas_core::logging::log_dir(&default_data_dir()?))
}
