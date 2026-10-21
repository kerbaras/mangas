//! Sending a finished EPUB to the Kindle, shared by `mangas kindle --send`,
//! `mangas export --send` and `mangas send`.

use std::io::Write;
use std::path::Path;

use mangas_core::library::{self, Library};
use mangas_core::send::password;
use mangas_core::send::{email, usb, SendVia};

use super::export_report::error_class;
use super::settings_email::warn_unencrypted;
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
/// user gave one. Returns the line saying where it went, for the caller to print.
pub async fn deliver(
    lib: &Library,
    file: &Path,
    title: &str,
    via: SendArg,
    kindle: Option<&Path>,
) -> anyhow::Result<String> {
    let (to, line) = match via {
        SendArg::Email => {
            let store = password::system_store();
            let settings = password::email_settings(lib, &*store)?;
            let in_file = !library::get_settings(lib)?.kindle_email.password.is_empty();
            if in_file {
                let why = store.get().err().map(|e| e.to_string());
                warn_unencrypted(
                    why.as_deref()
                        .unwrap_or("the system keychain didn't take it"),
                );
            }
            let sent = email::send_book(&settings, file, title).await?;
            let line = format!(
                "emailed {} ({:.1} MB) to {}",
                file.display(),
                mb(sent.bytes),
                sent.to
            );
            (sent.to, line)
        }
        SendArg::Usb => {
            let kindle = usb::choose(kindle, usb::scan_kindles())?;
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
                return Ok(format!(
                    "already on {}: {}",
                    kindle.name,
                    copied.path.display()
                ));
            }
            let line = format!("copied to {} ({})", copied.path.display(), kindle.name);
            (kindle.name, line)
        }
    };
    if let Err(e) = library::record_sent(lib, file, title, via.into(), &to) {
        eprintln!("warning: could not record the delivery: {e}");
    }
    Ok(line)
}

fn print_copy(done: u64, total: u64) {
    eprint!("\r  copying: {:.1}/{:.1} MB", mb(done), mb(total));
    let _ = std::io::stderr().flush();
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1e6
}

/// Sends one written book, saying on stderr straight away when it couldn't be sent, so a
/// later failure can't hide it. Returns [`deliver`]'s line; the error is that message.
pub async fn try_deliver(
    lib: &Library,
    file: &Path,
    title: &str,
    via: SendArg,
    kindle: Option<&Path>,
) -> Result<String, Unsent> {
    let e = match deliver(lib, file, title, via, kindle).await {
        Ok(line) => return Ok(line),
        Err(e) => e,
    };
    let unsent = not_sent(file, via, kindle, &e);
    eprintln!("{}", unsent.message);
    Err(unsent)
}

/// Between the file and the error in a [`not_sent`] line.
pub const NOT_SENT: &str = " was written but not sent: ";

/// A written book that wasn't sent: the [`not_sent`] line, and the class of its error (classed
/// from the error itself, so no path in the line can change it).
pub struct Unsent {
    pub message: String,
    pub class: Option<&'static str>,
}

/// "<file> was written but not sent: <why>", with the command that sends that same file again:
/// retrying a delivery never exports the book again. `kindle` is the USB mount it was sent to.
/// A Kindle connected over MTP only is offered email instead: a USB retry fails the same way.
pub fn not_sent(file: &Path, via: SendArg, kindle: Option<&Path>, e: &anyhow::Error) -> Unsent {
    let over_mtp = matches!(
        e.downcast_ref(),
        Some(mangas_core::Error::KindleOverMtp { .. })
    );
    let via = match (via, kindle) {
        _ if over_mtp => "email".to_string(),
        (SendArg::Email, _) => "email".to_string(),
        (SendArg::Usb, None) => "usb".to_string(),
        (SendArg::Usb, Some(mount)) => format!("usb --kindle \"{}\"", mount.display()),
    };
    let why = crate::exit::describe(e);
    let message = format!(
        "{}{NOT_SENT}{why}\n  send it again with: mangas send --via {via} \"{}\"",
        file.display(),
        file.display()
    );
    Unsent {
        class: error_class(&why),
        message,
    }
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
