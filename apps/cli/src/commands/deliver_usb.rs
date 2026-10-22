//! The USB half of [`super::deliver`]: find the Kindle, ask the format table, then copy.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use mangas_core::ml::DetectorReport;
use mangas_core::send::usb::{self, Sideload, UsbVerdict};

use super::deliver::SendArg;
use super::export_report::{Item, Report};

/// A book copied (or found already) on a Kindle, and the table's verdict for it.
pub struct UsbSent {
    pub to: String,
    pub path: PathBuf,
    pub already_there: bool,
    pub verdict: UsbVerdict,
}

/// Copies `file` to the Kindle at `kindle` (else the one detected) when the format table lets
/// it: a refused format never, one that warns only with `force` (the warning still goes to
/// stderr).
pub fn copy(file: &Path, kindle: Option<&Path>, force: bool) -> anyhow::Result<UsbSent> {
    let kindle = usb::choose(kindle, usb::scan_kindles())?;
    let verdict = usb::usb_verdict(file, &kindle.name);
    usb::admit(&verdict, force)?;
    if let (Sideload::Warn, Some(reason)) = (verdict.decision, &verdict.reason) {
        eprintln!("warning: {reason}");
    }
    let mut shown = false;
    let copied = usb::copy_to_kindle(file, &kindle, &mut |done, total| {
        shown = true;
        print_copy(done, total);
    });
    if shown {
        eprintln!();
    }
    let copied = copied?;
    Ok(UsbSent {
        to: kindle.name,
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

/// What to run after the table stopped `file`: `--force` for a format that only warns (and
/// Email for an EPUB); nothing for a refused one, whose reason already says to export a PDF.
/// `via` is the `--via` value that reached the same Kindle.
pub fn next_step(verdict: &UsbVerdict, file: &Path, via: &str) -> String {
    if verdict.decision != Sideload::Warn {
        return String::new();
    }
    let file = file.display();
    let email = match verdict.format.as_str() {
        "EPUB" => format!("\n  or email it: mangas send --via email \"{file}\""),
        _ => String::new(),
    };
    format!("\n  copy it anyway with: mangas send --via {via} --force \"{file}\"{email}")
}

/// Before an export that `send`s over USB: stops it, writing nothing, when the detected Kindle
/// won't open the `ext` format, or may not and `--force` wasn't given. With `json` the report
/// still prints, listing `books` (what the export would have written) in `not_written`. With no
/// Kindle found the export goes ahead and its send says so.
pub fn precheck(
    to: Option<(SendArg, Option<&Path>, bool)>,
    ext: &str,
    books: &[PathBuf],
    json: bool,
) -> anyhow::Result<()> {
    let Some((SendArg::Usb, kindle, force)) = to else {
        return Ok(());
    };
    let Ok(found) = usb::choose(kindle, usb::scan_kindles()) else {
        return Ok(());
    };
    let verdict = usb::usb_verdict(&Path::new("book").with_extension(ext), &found.name);
    let Err(e) = usb::admit(&verdict, force) else {
        return Ok(());
    };
    if json {
        not_copied(&verdict, books).print()?;
    }
    Err(match verdict.decision {
        Sideload::Warn => anyhow!("{e}\n  to write and copy it anyway, add --force"),
        _ => e.into(),
    })
}

/// The `--json` report of an export the format table stopped: every book in `not_written`, with
/// the verdict's reason and `usb_refused` (CBZ) or `usb_warn` (EPUB: `--force` would copy it).
fn not_copied(verdict: &UsbVerdict, books: &[PathBuf]) -> Report {
    let error = Some(match verdict.decision {
        Sideload::Refused => "usb_refused",
        _ => "usb_warn",
    });
    let mut report = Report::new(DetectorReport::off());
    report.not_written.extend(books.iter().map(|path| Item {
        path: path.clone(),
        reason: verdict.reason.clone().unwrap_or_default(),
        error,
    }));
    report
}

fn print_copy(done: u64, total: u64) {
    eprint!("\r  copying: {:.1}/{:.1} MB", mb(done), mb(total));
    let _ = std::io::stderr().flush();
}

pub fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1e6
}
