//! The `--format` flag of the export commands.

use mangas_core::export::Format;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum FormatArg {
    Epub,
    /// Fixed-layout KF8, the Kindle format to copy over USB
    Azw3,
    Cbz,
    Pdf,
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Epub => Format::Epub,
            FormatArg::Azw3 => Format::Azw3,
            FormatArg::Cbz => Format::Cbz,
            FormatArg::Pdf => Format::Pdf,
        }
    }
}
