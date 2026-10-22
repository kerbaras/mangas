//! `mangas send --json`: one object saying whether the book went, where, and what the USB
//! format table decided about it.

use std::path::{Path, PathBuf};

use mangas_core::send::usb::UsbVerdict;
use serde::Serialize;

use super::deliver::{Delivered, SendArg};
use super::deliver_usb::{over_mtp, stopped_by};

/// `usb` is the format table's verdict (`decision`: "supported", "warn" or "refused", with the
/// `format`, the `device` and the `reason`, also for a Kindle on MTP only); null for email, or
/// when no Kindle was found.
#[derive(Serialize)]
pub struct SendReport {
    pub file: PathBuf,
    pub via: &'static str,
    pub sent: bool,
    pub to: Option<String>,
    pub path: Option<PathBuf>,
    pub already_there: bool,
    pub usb: Option<UsbVerdict>,
    pub error: Option<String>,
}

impl SendReport {
    pub fn new(file: &Path, via: SendArg, result: &anyhow::Result<Delivered>) -> Self {
        let via = match via {
            SendArg::Email => "email",
            SendArg::Usb => "usb",
        };
        let base = SendReport {
            file: file.to_path_buf(),
            via,
            sent: false,
            to: None,
            path: None,
            already_there: false,
            usb: None,
            error: None,
        };
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
                let usb = stopped_by(e).cloned().or_else(|| over_mtp(e, file));
                SendReport {
                    to: usb.as_ref().map(|v| v.device.clone()),
                    usb,
                    error: Some(crate::exit::describe(e)),
                    ..base
                }
            }
        }
    }
}
