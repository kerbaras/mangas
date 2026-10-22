//! `mangas send`: email or copy a book to the Kindle (over USB only in a format it opens), test the email settings, or list the
//! Kindles connected over USB.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use clap::Args as ClapArgs;
use mangas_core::library;
use mangas_core::send::{email, password, usb};

use super::deliver::{deliver, say, SendArg};
use super::deliver_usb::{mtp_step, next_step, stopped_by};
use super::eject::{after_send, eject_kindle, ejected_line};
use super::send_args::SendArgs;
use super::send_report::SendReport;
use crate::output::{outln, print_json};
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Book to send
    pub file: Option<PathBuf>,
    /// How to send it
    #[arg(long, value_enum, default_value_t = SendArg::Email)]
    pub via: SendArg,
    /// Mount point of the Kindle for `--via usb` [default: detected]
    #[arg(long)]
    pub kindle: Option<PathBuf>,
    /// Title recorded in the export history [default: the file name]
    #[arg(long)]
    pub title: Option<String>,
    /// Send a test email to the sender address instead of a book
    #[arg(long, conflicts_with = "file")]
    pub test: bool,
    /// List the Kindles connected over USB
    #[arg(long, conflicts_with_all = ["file", "test"])]
    pub devices: bool,
    /// With `--via usb`, copy a file that is no export format (the Kindle may not open it); EPUB and CBZ are never copied
    #[arg(long, conflicts_with_all = ["test", "devices"])]
    pub force: bool,
    /// After `--via usb`, flush and eject the Kindle so it can be unplugged; without a file,
    /// `--via usb --eject` ejects the connected Kindle (`--kindle` picks one). No-op for email
    #[arg(long, conflicts_with_all = ["test", "devices"])]
    pub eject: bool,
    /// Print one JSON object instead of the line: `file`, `via`, `sent`, `to`, `path`,
    /// `already_there`, `error`, `usb` (`decision` "supported", "warn" or "refused",
    /// `format`, `device`, `reason`; null for email) and `eject` (`attempted`, `ok`, `error`),
    /// also when it fails (non-zero exit); `file` is null without a book
    #[arg(long, conflicts_with_all = ["test", "devices"])]
    pub json: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if args.devices {
        return list_devices();
    }
    if args.test {
        let lib = library::open(&paths::library_db()?)?;
        let settings = password::email_settings(&lib, &*password::system_store())?;
        let to = email::send_test(&settings).await?;
        return Ok(outln!("test email sent to {to}")?);
    }
    let Some(given) = args.file.clone() else {
        return no_book(&args);
    };
    let (lib, file) = match open_book(&given) {
        Ok(opened) => opened,
        Err(e) => return stopped(&args, Some(&given), e),
    };
    let title = args.title.clone().unwrap_or_else(|| {
        file.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let kindle = args.kindle.as_deref();
    let sent = deliver(&lib, &file, &title, args.via, kindle, args.force).await;
    if let (Ok(done), false) = (&sent, args.json) {
        outln!("{}", done.line)?;
    }
    let copied = match &sent {
        Ok(_) => std::slice::from_ref(&file),
        Err(_) => &[],
    };
    let eject = after_send(&eject_args(&args), copied, args.json);
    if args.json {
        let mut report = SendReport::new(Some(&file), args.via, &sent);
        report.eject = eject.clone();
        print_json(&report)?;
    }
    sent.map_err(|e| match (e.downcast_ref(), stopped_by(&e)) {
        (Some(mangas_core::Error::KindleOverMtp { .. }), _) => {
            anyhow::anyhow!("{e}{}", mtp_step(&file))
        }
        (_, Some(verdict)) => {
            let next = next_step(verdict, &file, "usb");
            anyhow::anyhow!("{e}{next}")
        }
        _ => e,
    })?;
    eject.result()
}

/// The library and `file` as an absolute path, once it is known to be a file.
fn open_book(file: &Path) -> anyhow::Result<(library::Library, PathBuf)> {
    if !file.is_file() {
        bail!("{} is not a file", file.display());
    }
    let lib = library::open(&paths::library_db()?)?;
    Ok((lib, std::path::absolute(file)?))
}

/// Without a book, `--via usb --eject` ejects the Kindle; anything else is an error.
fn no_book(args: &Args) -> anyhow::Result<()> {
    let ejected = match (args.eject, args.via) {
        (false, _) => return stopped(args, None, anyhow!(NO_BOOK)),
        (true, SendArg::Email) => return stopped(args, None, anyhow!(EJECT_NEEDS_USB)),
        (true, SendArg::Usb) => eject_kindle(args.kindle.as_deref(), &[]),
    };
    if args.json {
        print_json(&SendReport::ejected(args.via, &ejected))?;
    }
    say(&ejected_line(&ejected?), args.json)
}

/// A send that `e` stopped before delivering: with `--json` its report, then `e` as the exit.
fn stopped(args: &Args, file: Option<&Path>, e: anyhow::Error) -> anyhow::Result<()> {
    let sent = Err(e);
    if args.json {
        print_json(&SendReport::new(file, args.via, &sent))?;
    }
    sent.map(drop)
}

const NO_BOOK: &str = "no book given (or use --test / --devices)";
const EJECT_NEEDS_USB: &str = "--eject without a book ejects a Kindle over USB: add --via usb";

/// `send`'s flags in the shape `export`/`kindle --send` use for `--eject`.
fn eject_args(args: &Args) -> SendArgs {
    SendArgs {
        send: Some(args.via),
        kindle: args.kindle.clone(),
        force: args.force,
        eject: args.eject,
    }
}

fn list_devices() -> anyhow::Result<()> {
    let found = usb::scan_kindles();
    match (found.volumes.is_empty(), found.mtp.first()) {
        (false, _) => {}
        (true, Some(device)) => eprintln!("{} {}\n  {EMAIL_INSTEAD}", device.name, usb::MTP_HINT),
        (true, None) => eprintln!("Device not found: connect your Kindle over USB and unlock it"),
    }
    for k in found.volumes {
        outln!("{}\t{}", k.name, k.documents.display())?;
    }
    Ok(())
}

/// How to send a book when the Kindle is connected over MTP only.
pub const EMAIL_INSTEAD: &str = "send it by email instead: mangas send --via email <file>";
