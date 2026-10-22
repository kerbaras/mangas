//! The USB half of [`super::deliver`]: find the Kindle, ask the format table, then copy.

use std::io::Write;
use std::path::{Path, PathBuf};

use mangas_core::send::email;
use mangas_core::send::usb::{self, Sideload, UsbVerdict};

/// A book copied (or found already) on a Kindle, and the table's verdict for it.
pub struct UsbSent {
    pub to: String,
    pub path: PathBuf,
    pub already_there: bool,
    pub verdict: UsbVerdict,
}

/// Copies `file` to the Kindle at `kindle` (else the one detected) when the format table lets
/// it: a refused format never, one that warns only with `force` (the warning still goes to
/// stderr). The table speaks first, so a Kindle on MTP only still refuses a CBZ as a CBZ, before
/// anything is uploaded. Where this build copies over MTP (macOS/Windows, feature `mtp`), a
/// Kindle with no mounted folder is copied to directly; Ctrl-C cancels the copy.
pub fn copy(file: &Path, kindle: Option<&Path>, force: bool) -> anyhow::Result<UsbSent> {
    let chosen = usb::target(kindle, usb::scan_kindles());
    let verdict = usb::target_name(&chosen).map(|name| usb::usb_verdict(file, name));
    if let Some(verdict) = &verdict {
        usb::admit(verdict, force)?;
    }
    let kindle = chosen?;
    let verdict = verdict.unwrap_or_else(|| usb::usb_verdict(file, kindle.name()));
    if let (Sideload::Warn, Some(reason)) = (verdict.decision, &verdict.reason) {
        eprintln!("warning: {reason}");
    }
    let mut shown = false;
    let copied = super::copy_cancel::cancellable(|cancelled| {
        usb::deliver(file, &kindle, cancelled, &mut |done, total| {
            shown = true;
            print_copy(done, total);
        })
    });
    if shown {
        eprintln!();
    }
    let copied = copied?;
    Ok(UsbSent {
        to: kindle.name().to_string(),
        path: copied.path,
        already_there: copied.already_there,
        verdict,
    })
}

/// The verdict that stopped a USB copy, when the format table did.
pub fn stopped_by(e: &anyhow::Error) -> Option<&UsbVerdict> {
    match e.downcast_ref() {
        Some(mangas_core::Error::NotOnKindle(verdict)) => Some(verdict),
        _ => None,
    }
}

/// The table's verdict for `file` on the Kindle `e` found over MTP only (it wasn't stopped by
/// the table, only by MTP), so `send --json` still reports the decision.
pub fn over_mtp(e: &anyhow::Error, file: &Path) -> Option<UsbVerdict> {
    match e.downcast_ref() {
        Some(mangas_core::Error::KindleOverMtp { device }) => Some(usb::usb_verdict(file, device)),
        _ => None,
    }
}

/// What to run when the Kindle is on MTP only: Email when it takes `file`'s format, otherwise
/// an export in a format it does take (a USB retry fails the same way).
pub fn mtp_step(file: &Path) -> String {
    match email::emailable(&file.to_string_lossy()) {
        true => format!(
            "\n  send it by email instead: mangas send --via email \"{}\"",
            file.display()
        ),
        false => "\n  to send it by email, export the book as PDF or EPUB".to_string(),
    }
}

/// What to run after the table stopped `file`: `--force` for a file that only warns, Email for
/// an EPUB (its route to a Kindle), nothing for a CBZ, whose reason already says what to export.
/// `via` is the `--via` value that reached the same Kindle.
pub fn next_step(verdict: &UsbVerdict, file: &Path, via: &str) -> String {
    let file = file.display();
    match (verdict.decision, verdict.format.as_str()) {
        (Sideload::Warn, _) => {
            format!("\n  copy it anyway with: mangas send --via {via} --force \"{file}\"")
        }
        (Sideload::Refused, "EPUB") => {
            format!("\n  send it by email instead: mangas send --via email \"{file}\"")
        }
        _ => String::new(),
    }
}

fn print_copy(done: u64, total: u64) {
    eprint!("\r  copying: {:.1}/{:.1} MB", mb(done), mb(total));
    let _ = std::io::stderr().flush();
}

pub fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1e6
}
