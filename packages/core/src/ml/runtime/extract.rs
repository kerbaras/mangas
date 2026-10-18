//! Pulls one file out of an in-memory `.tgz` or `.zip` archive.

use std::io::{Cursor, Read};

use flate2::read::GzDecoder;

use super::Archive;
use crate::domain::{Error, Result};

pub fn extract_member(archive: &[u8], kind: Archive, member: &str) -> Result<Vec<u8>> {
    match kind {
        Archive::Tgz => from_tgz(archive, member),
        Archive::Zip => from_zip(archive, member),
    }
}

fn normalize(path: &str) -> &str {
    path.trim_start_matches("./")
}

fn missing(member: &str) -> Error {
    Error::Other(format!("archive has no {member}"))
}

fn from_tgz(archive: &[u8], member: &str) -> Result<Vec<u8>> {
    let mut tar = tar::Archive::new(GzDecoder::new(archive));
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();
        if entry.header().entry_type().is_file() && normalize(&path) == normalize(member) {
            let mut out = Vec::new();
            entry.read_to_end(&mut out)?;
            return Ok(out);
        }
    }
    Err(missing(member))
}

fn from_zip(archive: &[u8], member: &str) -> Result<Vec<u8>> {
    let zip_err = |e: zip::result::ZipError| Error::Other(format!("zip: {e}"));
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).map_err(zip_err)?;
    let mut file = zip.by_name(member).map_err(|_| missing(member))?;
    let mut out = Vec::new();
    file.read_to_end(&mut out)?;
    Ok(out)
}
