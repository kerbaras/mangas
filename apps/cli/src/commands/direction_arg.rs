//! The `--direction` flag of the export commands.

use mangas_core::domain::ReadingDirection;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum DirectionArg {
    /// Left to right (comics, manhwa, webtoons)
    Ltr,
    /// Right to left (manga)
    Rtl,
}

impl From<DirectionArg> for ReadingDirection {
    fn from(d: DirectionArg) -> Self {
        match d {
            DirectionArg::Ltr => ReadingDirection::Ltr,
            DirectionArg::Rtl => ReadingDirection::Rtl,
        }
    }
}
