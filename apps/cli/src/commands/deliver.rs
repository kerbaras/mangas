//! Sending a finished EPUB to the Kindle, shared by `mangas kindle --send`,
//! `mangas export --send` and `mangas send`.

use std::io::Write;
use std::path::Path;

use mangas_core::library::{self, Library};
use mangas_core::send::password::{self, Storage};
use mangas_core::send::{email, usb, SendVia};

use super::settings_email::warn_unencrypted;
use crate::output::outln;
use crate::Exit;

/// Delivery path picked on the command line.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum SendArg {
    /// Email it to the Kindle address in `mangas settings email`
    Email,
    /// Copy it to a Kindle mounted over USB
    Usb,
}

impl From<SendArg> for SendVia {
    fn from(a: SendArg) -> Self {
        match a {
            SendArg::Email => SendVia::Email,
            SendArg::Usb => SendVia::Usb,
        }
    }
}

/// Sends `file` and stamps it in the export history. `kindle` is the USB mount point when the
/// user gave one.
pub async fn deliver(
    lib: &Library,
    file: &Path,
    title: &str,
    via: SendArg,
    kindle: Option<&Path>,
) -> anyhow::Result<()> {
    let to = match via {
        SendArg::Email => {
            let store = password::system_store();
            let settings = password::email_settings(lib, &*store)?;
            if !settings.password.is_empty() && password::storage(lib, &*store)? == Storage::File {
                let why = store.get().err();
                warn_unencrypted(
                    why.as_deref()
                        .unwrap_or("the system keychain didn't take it"),
                );
            }
            let sent = email::send_book(&settings, file, title).await?;
            outln!(
                "emailed {} ({:.1} MB) to {}",
                file.display(),
                mb(sent.bytes),
                sent.to
            )?;
            sent.to
        }
        SendArg::Usb => {
            let kindle = usb::choose(kindle, usb::find_kindles())?;
            let mut shown = false;
            let copied = usb::copy_to_kindle(file, &kindle, &mut |done, total| {
                shown = true;
                print_copy(done, total);
            });
            if shown {
                eprintln!();
            }
            let copied = copied?;
            if copied.already_there {
                outln!("already on {}: {}", kindle.name, copied.path.display())?;
                return Ok(());
            }
            outln!("copied to {} ({})", copied.path.display(), kindle.name)?;
            kindle.name
        }
    };
    if let Err(e) = library::record_sent(lib, file, title, via.into(), &to) {
        eprintln!("warning: could not record the delivery: {e}");
    }
    Ok(())
}

fn print_copy(done: u64, total: u64) {
    eprint!("\r  copying: {:.1}/{:.1} MB", mb(done), mb(total));
    let _ = std::io::stderr().flush();
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1e6
}

/// Sends one written book, saying on stderr straight away when it couldn't be sent, so a
/// later failure can't hide it. Returns whether it was sent.
pub async fn try_deliver(
    lib: &Library,
    file: &Path,
    title: &str,
    via: SendArg,
    kindle: Option<&Path>,
) -> bool {
    let Err(e) = deliver(lib, file, title, via, kindle).await else {
        return true;
    };
    eprintln!("{} was written but not sent: {e:#}", file.display());
    false
}

/// Exit code 2 when some written books weren't sent (each was named as it failed).
pub fn unsent(failed: usize) -> anyhow::Result<()> {
    if failed == 0 {
        return Ok(());
    }
    Err(Exit {
        code: 2,
        message: format!("{failed} book(s) were written but not sent"),
    }
    .into())
}
