//! `mangas send --json`: one object saying whether the book went, where, and what the USB
//! format table decided about it.

use std::path::{Path, PathBuf};

use mangas_core::send::usb::UsbVerdict;
use serde::Serialize;

use mangas_core::send::eject::Ejected;

use super::deliver::{Delivered, SendArg};
use super::deliver_usb::{over_mtp, stopped_by};
use super::eject::EjectReport;

/// `usb` is the format table's verdict (`decision`: "supported", "warn" or "refused", with the
/// `format`, the `device` and the `reason`, also for a Kindle on MTP only); null for email, or
/// when no Kindle was found. `eject` is what `--eject` did after the copy. `file` is null when
/// no book was given (`--via usb --eject` alone ejects the Kindle).
#[derive(Serialize)]
pub struct SendReport {
    pub file: Option<PathBuf>,
    pub via: &'static str,
    pub sent: bool,
    pub to: Option<String>,
    pub path: Option<PathBuf>,
    pub already_there: bool,
    pub usb: Option<UsbVerdict>,
    pub error: Option<String>,
    pub eject: EjectReport,
}

impl SendReport {
    fn base(file: Option<&Path>, via: SendArg) -> Self {
        let via = match via {
            SendArg::Email => "email",
            SendArg::Usb => "usb",
        };
        SendReport {
            file: file.map(Path::to_path_buf),
            via,
            sent: false,
            to: None,
            path: None,
            already_there: false,
            usb: None,
            error: None,
            eject: EjectReport::default(),
        }
    }

    pub fn new(file: Option<&Path>, via: SendArg, result: &anyhow::Result<Delivered>) -> Self {
        let base = Self::base(file, via);
        match result {
            Ok(done) => SendReport {
                sent: true,
                to: Some(done.to.clone()),
                path: done.path.clone(),
                already_there: done.already_there,
                usb: done.usb.clone(),
                ..base
            },
            Err(e) => {
                let usb = stopped_by(e)
                    .cloned()
                    .or_else(|| file.and_then(|f| over_mtp(e, f)));
                SendReport {
                    to: usb.as_ref().map(|v| v.device.clone()),
                    usb,
                    error: Some(crate::exit::describe(e)),
                    ..base
                }
            }
        }
    }

    /// `--via usb --eject` without a book: nothing was sent, the Kindle was `ejected` (or not).
    pub fn ejected(via: SendArg, ejected: &anyhow::Result<Ejected>) -> Self {
        SendReport {
            to: ejected.as_ref().ok().map(|done| done.device.clone()),
            eject: EjectReport::of(ejected),
            ..Self::base(None, via)
        }
    }
}
