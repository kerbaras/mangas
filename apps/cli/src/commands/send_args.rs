//! `--send`, `--kindle` and `--eject`, shared by `mangas export` and `mangas kindle`.

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;

use super::deliver::SendArg;

#[derive(ClapArgs, Debug, Clone, Default)]
pub struct SendArgs {
    /// After writing, email the book to the Kindle or copy it to one over USB
    #[arg(long, value_enum)]
    pub send: Option<SendArg>,
    /// Mount point of the Kindle for `--send usb` [default: detected]
    #[arg(long)]
    pub kindle: Option<PathBuf>,
    /// With `--send usb`, copy a file that is no export format (the Kindle may not open it);
    /// EPUB and CBZ are never copied
    #[arg(long, requires = "send")]
    pub force: bool,
    /// After `--send usb`, flush and eject the Kindle so it can be unplugged (no-op for email)
    #[arg(long)]
    pub eject: bool,
}

impl SendArgs {
    /// How to send each book and, over USB, to which Kindle and whether to `--force` the format.
    pub fn target(&self) -> Option<(SendArg, Option<&Path>, bool)> {
        self.send
            .map(|via| (via, self.kindle.as_deref(), self.force))
    }
}
