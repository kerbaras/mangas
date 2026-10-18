use std::path::PathBuf;

use serde::Serialize;

/// Where the app keeps its files. `data_dir` is the user-chosen folder (chapters and detector
/// models); the database, cover cache and logs stay in the platform app-data folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Folders {
    pub data_dir: PathBuf,
    pub database: PathBuf,
    pub covers: PathBuf,
    pub logs: PathBuf,
}

impl Folders {
    pub fn chapters(&self) -> PathBuf {
        self.data_dir.join("chapters")
    }

    pub fn models(&self) -> PathBuf {
        self.data_dir.join("models")
    }
}

/// The subfolders of a data folder that belong to the app and move with it.
pub(crate) const DATA_SUBDIRS: [&str; 2] = ["chapters", "models"];
