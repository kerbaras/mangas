//! `--eject`: the Kindle a USB delivery copied to, flushed and ejected (core `send::eject`).

use std::path::{Path, PathBuf};

use mangas_core::send::eject::{self, Ejected};
use mangas_core::send::usb;
use serde::Serialize;

use super::deliver::{say, SendArg};
use super::send_args::SendArgs;

/// Said when `--eject` comes with an email delivery.
pub const NOTHING_TO_EJECT: &str = "--eject: nothing to eject over email";

/// `eject` in the `--json` report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct EjectReport {
    pub attempted: bool,
    pub ok: bool,
    pub error: Option<String>,
}

impl EjectReport {
    /// What an eject that ran says in `--json`.
    pub fn of(ejected: &anyhow::Result<Ejected>) -> Self {
        EjectReport {
            attempted: true,
            ok: ejected.is_ok(),
            error: ejected.as_ref().err().map(crate::exit::describe),
        }
    }

    /// The eject's message as the command's error (exit 1) when it failed.
    pub fn result(&self) -> anyhow::Result<()> {
        match &self.error {
            Some(e) => Err(anyhow::anyhow!("{e}")),
            None => Ok(()),
        }
    }

    /// The command's error when the run `stop`ped early (why), the eject's message after it.
    pub fn result_after(&self, stop: Option<String>) -> anyhow::Result<()> {
        match (stop, &self.error) {
            (Some(stop), Some(e)) => Err(anyhow::anyhow!("{stop}\n{e}")),
            (Some(stop), None) => Err(anyhow::anyhow!("{stop}")),
            (None, _) => self.result(),
        }
    }
}

/// With `--eject`, ejects the Kindle `sent` (the source files) were copied to over USB, saying
/// so on stdout (stderr when stdout carries JSON). Email has nothing to eject. With nothing
/// copied (a refused or failed send) it still ejects the Kindle, when one is mounted.
pub fn after_send(to: &SendArgs, sent: &[PathBuf], json: bool) -> EjectReport {
    if !to.eject {
        return EjectReport::default();
    }
    if !matches!(to.send, Some(SendArg::Usb)) {
        eprintln!("{NOTHING_TO_EJECT}");
        return EjectReport::default();
    }
    if sent.is_empty() && usb::choose(to.kindle.as_deref(), usb::scan_kindles()).is_err() {
        eprintln!("--eject: no mounted Kindle to eject");
        return EjectReport::default();
    }
    let ejected = eject_kindle(to.kindle.as_deref(), sent);
    if let Ok(done) = &ejected {
        drop(say(&ejected_line(done), json));
    }
    EjectReport::of(&ejected)
}

/// Flushes the books named like `files` in the Kindle's documents folder, then ejects the
/// Kindle at `kindle` (when `None`, the connected one holding them).
pub fn eject_kindle(kindle: Option<&Path>, files: &[PathBuf]) -> anyhow::Result<Ejected> {
    let volume = usb::choose_holding(kindle, usb::scan_kindles(), files)?;
    Ok(eject::eject(&volume, &usb::on_kindle(&volume, files))?)
}

pub fn ejected_line(done: &Ejected) -> String {
    format!("ejected {}: it's safe to unplug", done.device)
}
