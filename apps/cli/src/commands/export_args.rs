//! The `--format` flag of the export commands.

use mangas_core::export::Format;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum FormatArg {
    Epub,
    Cbz,
    Pdf,
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Epub => Format::Epub,
            FormatArg::Cbz => Format::Cbz,
            FormatArg::Pdf => Format::Pdf,
        }
    }
}
