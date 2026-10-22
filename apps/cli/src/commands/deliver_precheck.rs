//! Before an export that sends, and before `--detectors` loads anything (so the report's
//! `detectors` is truly `off`): stops it, writing nothing, when the delivery can't take the
//! book. Email asks core's [`email::email_refusal`]; USB asks the format table for the Kindle
//! found, and a Kindle on MTP only stops it too, since nothing can be copied to it. With no
//! Kindle at all the export goes ahead and its send says so.

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use mangas_core::ml::DetectorReport;
use mangas_core::send::email;
use mangas_core::send::usb::{self, Sideload};

use super::deliver::SendArg;
use super::export_report::{Item, Report};

/// Why the export stops: the reason (core's words), its `--json` class, and the error to exit
/// with (the reason and the flags that get the book there).
struct Stop {
    reason: String,
    class: &'static str,
    error: anyhow::Error,
}

/// Stops an export sending `ext` books by `to` (via, Kindle mount, `--force`) when that
/// delivery can't take them. With `json` the report still prints, listing `books` (what the
/// export would have written) in `not_written` with the reason and its class.
pub fn precheck(
    to: Option<(SendArg, Option<&Path>, bool)>,
    ext: &str,
    books: &[PathBuf],
    json: bool,
) -> anyhow::Result<()> {
    let Some((via, kindle, force)) = to else {
        return Ok(());
    };
    let stop = match via {
        SendArg::Email => by_email(ext),
        SendArg::Usb => over_usb(kindle, ext, force),
    };
    let Some(stop) = stop else {
        return Ok(());
    };
    if json {
        not_written(&stop, books).print()?;
    }
    Err(stop.error)
}

/// Send to Kindle by email takes EPUB and PDF only (core's rule), so an AZW3 or CBZ export
/// for Email stops. USB is pointed at only for a format the table copies to a Kindle.
fn by_email(ext: &str) -> Option<Stop> {
    let book = Path::new("book").with_extension(ext);
    let reason = email::email_refusal(&book.to_string_lossy())?;
    let usb = match usb::usb_verdict(&book, "Kindle").decision {
        Sideload::Supported => "; for USB, use --send usb",
        _ => "",
    };
    let error = anyhow!(
        "{reason}\n  to email it, export with --format epub (or leave --format out on a Kindle \
         profile){usb}"
    );
    Some(Stop {
        reason,
        class: "email_refused",
        error,
    })
}

/// The format table's verdict for the Kindle found (mounted or MTP only), then MTP itself.
fn over_usb(kindle: Option<&Path>, ext: &str, force: bool) -> Option<Stop> {
    let chosen = usb::choose(kindle, usb::scan_kindles());
    let book = Path::new("book").with_extension(ext);
    let verdict = usb::usb_verdict(&book, usb::chosen_name(&chosen)?);
    if let Err(e) = usb::admit(&verdict, force) {
        let error = match (verdict.decision, ext) {
            (Sideload::Warn, _) => anyhow!("{e}\n  to write and copy it anyway, add --force"),
            (Sideload::Refused, "epub") => anyhow!(
                "{e}\n  for USB, export with --format azw3 (or leave --format out); for Email, \
                 use --send email"
            ),
            _ => e.into(),
        };
        let class = match verdict.decision {
            Sideload::Refused => "usb_refused",
            _ => "usb_warn",
        };
        let reason = verdict.reason.unwrap_or_default();
        return Some(Stop {
            reason,
            class,
            error,
        });
    }
    let e = chosen.err()?;
    let next = match email::emailable(&book.to_string_lossy()) {
        true => "\n  send it by email instead: add --send email in place of --send usb",
        false => "\n  send it by email instead: export with --format epub --send email",
    };
    Some(Stop {
        reason: e.to_string(),
        class: "kindle_over_mtp",
        error: anyhow!("{e}{next}"),
    })
}

/// The `--json` report of a stopped export: every book in `not_written`.
fn not_written(stop: &Stop, books: &[PathBuf]) -> Report {
    let mut report = Report::new(DetectorReport::off());
    report.not_written.extend(books.iter().map(|path| Item {
        path: path.clone(),
        reason: stop.reason.clone(),
        error: Some(stop.class),
    }));
    report
}
