use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::domain::{Error, Result};

/// Packs the page files of `dir` (sorted by name, stored uncompressed) into the CBZ at `out`.
/// Returns the number of pages archived.
pub fn write_cbz(dir: &Path, out: &Path) -> Result<usize> {
    let pages = page_files(dir)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    crate::zip_out::write_zip(File::create(out)?, "cbz", |writer| {
        for name in &pages {
            writer
                .start_file(name.as_str(), options)
                .map_err(zip_error)?;
            io::copy(&mut File::open(dir.join(name))?, writer)?;
        }
        Ok(())
    })?;
    Ok(pages.len())
}

/// Image files directly inside `dir`, sorted so `0001.jpg` comes before `0002.png`.
fn page_files(dir: &Path) -> Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.') && is_image(name))
        .collect();
    names.sort();
    Ok(names)
}

pub(crate) const IMAGE_EXTENSIONS: [&str; 6] = ["jpg", "jpeg", "png", "gif", "webp", "avif"];

pub(crate) fn is_image(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// `<dir>.cbz` next to the chapter directory, keeping the whole directory name
/// (`2.5` becomes `2.5.cbz`, not `2.cbz`).
pub fn cbz_path(dir: &Path) -> PathBuf {
    let mut name = dir.file_name().map(OsStr::to_os_string).unwrap_or_default();
    name.push(".cbz");
    dir.with_file_name(name)
}

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Other(format!("cbz: {e}"))
}
