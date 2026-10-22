//! Sending a finished EPUB to the Kindle, shared by `mangas kindle --send`,
//! `mangas export --send` and `mangas send`.

use std::path::{Path, PathBuf};

use mangas_core::library::{self, Library};
use mangas_core::send::password;
use mangas_core::send::usb::UsbVerdict;
use mangas_core::send::{email, SendVia};

use super::deliver_usb::{self, mb, stopped_by};
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

/// Where a book went: the line saying so, the address or Kindle, the file on the Kindle, and
/// the format table's verdict for a USB copy.
pub struct Delivered {
    pub line: String,
    pub to: String,
    pub path: Option<PathBuf>,
    pub already_there: bool,
    pub usb: Option<UsbVerdict>,
}

/// Sends `file` and stamps it in the export history. `kindle` is the USB mount point when the
/// user gave one; `force` copies a format the Kindle may not open (see `usb::SIDELOAD`).
pub async fn deliver(
    lib: &Library,
    file: &Path,
    title: &str,
    via: SendArg,
    kindle: Option<&Path>,
    force: bool,
) -> anyhow::Result<Delivered> {
    let done = match via {
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
            Delivered {
                line,
                to: sent.to,
                path: None,
                already_there: false,
                usb: None,
            }
        }
        SendArg::Usb => {
            let sent = deliver_usb::copy(file, kindle, force)?;
            let line = match sent.already_there {
                true => format!("already on {}: {}", sent.to, sent.path.display()),
                false => format!("copied to {} ({})", sent.path.display(), sent.to),
            };
            let done = Delivered {
                line,
                to: sent.to,
                path: Some(sent.path),
                already_there: sent.already_there,
                usb: Some(sent.verdict),
            };
            if done.already_there {
                return Ok(done);
            }
            done
        }
    };
    if let Err(e) = library::record_sent(lib, file, title, via.into(), &done.to) {
        eprintln!("warning: could not record the delivery: {e}");
    }
    Ok(done)
}

/// Sends one written book, saying on stderr straight away when it couldn't be sent, so a
/// later failure can't hide it. Returns [`deliver`]'s line; the error is that message.
pub async fn try_deliver(
    lib: &Library,
    file: &Path,
    title: &str,
    via: SendArg,
    kindle: Option<&Path>,
    force: bool,
) -> Result<String, Unsent> {
    let e = match deliver(lib, file, title, via, kindle, force).await {
        Ok(done) => return Ok(done.line),
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
    let next = match stopped_by(e) {
        Some(verdict) => deliver_usb::next_step(verdict, file, &via),
        None => format!(
            "\n  send it again with: mangas send --via {via} \"{}\"",
            file.display()
        ),
    };
    let message = format!("{}{NOT_SENT}{why}{next}", file.display());
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
