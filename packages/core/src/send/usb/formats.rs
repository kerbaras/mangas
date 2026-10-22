//! Which export formats a Kindle opens when the file is copied into its `documents/` folder.
//!
//! From `docs/research/usb-delivery.md` ("Formats and sizes over USB", issue 2): calibre's
//! Kindle format lists name PDF and AZW3 but neither EPUB nor CBZ, and calibre never side-loads
//! either, so EPUB is refused over USB (#329): AZW3 is the Kindle format to copy, and Email is
//! the EPUB route, because Amazon converts EPUB on the way.

use std::path::Path;

use serde::Serialize;

use crate::domain::{Error, Result};

/// What a Kindle does with a file of a format copied onto it over USB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Sideload {
    /// Opens: copied as is.
    Supported,
    /// May not open: copied only once the user confirms.
    Warn,
    /// Doesn't open: never copied.
    Refused,
}

/// Export format (its file extension) → what a Kindle does with it side-loaded. The one table
/// every USB copy consults.
pub const SIDELOAD: [(&str, Sideload); 4] = [
    ("pdf", Sideload::Supported),
    ("azw3", Sideload::Supported),
    ("epub", Sideload::Refused),
    ("cbz", Sideload::Refused),
];

/// What a Kindle does with a side-loaded `.ext` file (case-insensitive); `None` for an
/// extension that isn't an export format.
pub fn sideload(ext: &str) -> Option<Sideload> {
    SIDELOAD
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(ext))
        .map(|&(_, s)| s)
}

/// The decision for copying one file to one Kindle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsbVerdict {
    pub decision: Sideload,
    /// The format as users name it: the extension upper-cased ("EPUB"), empty without one.
    pub format: String,
    /// The Kindle's name, as USB detection reports it.
    pub device: String,
    /// Why it may not or won't open and what to do instead; `None` when supported.
    pub reason: Option<String>,
}

/// What copying `file` onto `device` does, by [`SIDELOAD`]. A file that isn't an export format
/// warns: the Kindle may open it (calibre lists MOBI and TXT), but mangas can't tell.
pub fn usb_verdict(file: &Path, device: &str) -> UsbVerdict {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    let format = ext.to_ascii_uppercase();
    let instead = "export the book as PDF or AZW3";
    let (decision, reason) = match sideload(ext) {
        Some(Sideload::Supported) => (Sideload::Supported, None),
        Some(Sideload::Refused) if ext.eq_ignore_ascii_case("epub") => (
            Sideload::Refused,
            Some(format!(
                "{device} doesn't open an EPUB copied over USB. Export the book as AZW3 to \
                 copy it, or send the EPUB by Email (Send to Kindle converts it)."
            )),
        ),
        Some(Sideload::Refused) => (
            Sideload::Refused,
            Some(format!(
                "{device} doesn't open {format} files. To copy it over USB, {instead}."
            )),
        ),
        Some(Sideload::Warn) => (
            Sideload::Warn,
            Some(format!(
                "{device} may not open an {format} copied over USB. Email it instead (Send to \
                 Kindle converts {format}), or {instead}."
            )),
        ),
        None => {
            let what = match ext {
                "" => "a file without an extension".to_string(),
                _ => format!("a .{ext} file"),
            };
            let reason = format!(
                "{device} may not open {what}; mangas only knows its export formats. To be \
                 sure, {instead}."
            );
            (Sideload::Warn, Some(reason))
        }
    };
    UsbVerdict {
        decision,
        format,
        device: device.to_string(),
        reason,
    }
}

/// Whether the copy may go ahead: [`Error::NotOnKindle`] for a refused format, and for one that
/// warns unless the user confirmed (`force`).
pub fn admit(verdict: &UsbVerdict, force: bool) -> Result<()> {
    match verdict.decision {
        Sideload::Supported => Ok(()),
        Sideload::Warn if force => Ok(()),
        Sideload::Warn | Sideload::Refused => Err(Error::NotOnKindle(Box::new(verdict.clone()))),
    }
}
