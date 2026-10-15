use std::fs::File;
use std::io;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::domain::{Error, Result};

/// Packs the page files of `dir` (sorted by name, stored uncompressed) into the CBZ at `out`.
/// Returns the number of pages archived.
pub fn write_cbz(dir: &Path, out: &Path) -> Result<usize> {
    let pages = page_files(dir)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = ZipWriter::new(File::create(out)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for name in &pages {
        writer
            .start_file(name.as_str(), options)
            .map_err(zip_error)?;
        io::copy(&mut File::open(dir.join(name))?, &mut writer)?;
    }
    writer.finish().map_err(zip_error)?;
    Ok(pages.len())
}

/// Image files directly inside `dir`, sorted so `0001.jpg` comes before `0002.png`.
fn page_files(dir: &Path) -> Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.ends_with(".part") && !name.starts_with('.'))
        .collect();
    names.sort();
    Ok(names)
}

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Other(format!("cbz: {e}"))
}
