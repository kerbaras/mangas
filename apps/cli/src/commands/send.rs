//! `mangas send`: email or copy a book to the Kindle (over USB only in a format it opens), test the email settings, or list the
//! Kindles connected over USB.

use std::path::PathBuf;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::library;
use mangas_core::send::{email, password, usb};

use super::deliver::{deliver, SendArg};
use super::deliver_usb::{mtp_step, next_step, stopped_by};
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
    /// Print one JSON object instead of the line: `file`, `via`, `sent`, `to`, `path`,
    /// `already_there`, `error`, and `usb` (`decision` "supported", "warn" or "refused",
    /// `format`, `device`, `reason`; null for email)
    #[arg(long, conflicts_with_all = ["test", "devices"])]
    pub json: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if args.devices {
        return list_devices();
    }
    let lib = library::open(&paths::library_db()?)?;
    if args.test {
        let settings = password::email_settings(&lib, &*password::system_store())?;
        let to = email::send_test(&settings).await?;
        return Ok(outln!("test email sent to {to}")?);
    }
    let Some(file) = args.file else {
        bail!("no EPUB given (or use --test / --devices)");
    };
    if !file.is_file() {
        bail!("{} is not a file", file.display());
    }
    let file = std::path::absolute(&file)?;
    let title = args.title.unwrap_or_else(|| {
        file.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let kindle = args.kindle.as_deref();
    let sent = deliver(&lib, &file, &title, args.via, kindle, args.force).await;
    if args.json {
        print_json(&SendReport::new(&file, args.via, &sent))?;
    }
    let line = sent.map_err(|e| match (e.downcast_ref(), stopped_by(&e)) {
        (Some(mangas_core::Error::KindleOverMtp { .. }), _) => {
            anyhow::anyhow!("{e}{}", mtp_step(&file))
        }
        (_, Some(verdict)) => {
            let next = next_step(verdict, &file, "usb");
            anyhow::anyhow!("{e}{next}")
        }
        _ => e,
    })?;
    if !args.json {
        outln!("{}", line.line)?;
    }
    Ok(())
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
