//! `mangas send`: email or copy an EPUB to the Kindle, test the email settings, or list the
//! Kindles connected over USB.

use std::path::PathBuf;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::library;
use mangas_core::send::{email, password, usb};

use super::deliver::{deliver, SendArg};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// EPUB to send
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
    deliver(&lib, &file, &title, args.via, args.kindle.as_deref()).await
}

fn list_devices() -> anyhow::Result<()> {
    let found = usb::find_kindles();
    if found.is_empty() {
        eprintln!("Device not found: connect your Kindle over USB and unlock it");
    }
    for k in found {
        outln!("{}\t{}", k.name, k.documents.display())?;
    }
    Ok(())
}
