//! The `Exporter` contract: finished device pages plus book metadata in, one file out.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::azw3::Azw3Exporter;
use super::cbz::CbzExporter;
use super::epub::{ChapterEntry, EpubExporter, PageImage, Series};
use super::pdf::PdfExporter;
use super::PageSpool;
use super::Profile;
use crate::domain::{Error, Result};

/// What every format needs to know about the book besides its pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub id: String,
    pub title: String,
    /// The creators; empty when unknown, and then no format writes one.
    pub authors: Vec<String>,
    pub language: String,
    pub series: Option<Series>,
    pub right_to_left: bool,
    /// One entry per chapter that has pages, in reading order.
    pub chapters: Vec<ChapterEntry>,
    pub labels: Labels,
    /// `title` was typed by the user rather than generated.
    pub title_explicit: bool,
    /// A separate cover page; formats without one start with the first page.
    pub cover: Option<PageImage>,
}

/// What the library knows about the exported chapters; each field is set only when it holds
/// for the whole file (one series, one chapter, one volume).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels {
    pub series: Option<String>,
    /// The chapter's own title, for a single-chapter export.
    pub title: Option<String>,
    pub number: Option<String>,
    pub volume: Option<String>,
}

impl Labels {
    /// Fills the series and volume from the book's series (a volume export) when unknown.
    pub fn or_series(self, series: Option<&Series>) -> Labels {
        Labels {
            series: self.series.or_else(|| series.map(|s| s.name.clone())),
            volume: self
                .volume
                .or_else(|| series.and_then(|s| s.position.clone())),
            ..self
        }
    }
}

/// Writes processed pages as one file and returns the path written. An existing file is
/// replaced only when `replace` (atomically: a failed export keeps the old one); otherwise the
/// name gets ` (2)`, .... Implementations never leave a half-written file.
pub trait Exporter: Sync {
    fn name(&self) -> &'static str;
    /// File extension without the dot.
    fn extension(&self) -> &'static str;
    fn export(
        &self,
        pages: PageSpool,
        meta: &Meta,
        profile: &Profile,
        output: &Path,
        replace: bool,
    ) -> Result<PathBuf>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Epub,
    Azw3,
    Cbz,
    Pdf,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Epub, Format::Azw3, Format::Cbz, Format::Pdf];

    pub fn exporter(self) -> &'static dyn Exporter {
        match self {
            Format::Epub => &EpubExporter,
            Format::Azw3 => &Azw3Exporter,
            Format::Cbz => &CbzExporter,
            Format::Pdf => &PdfExporter,
        }
    }

    pub fn id(self) -> &'static str {
        self.exporter().extension()
    }

    /// The format a file name asks for, by extension (case-insensitive).
    pub fn of_path(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?;
        Format::ALL
            .into_iter()
            .find(|f| f.id().eq_ignore_ascii_case(ext))
    }

    /// Picks the format of an export: the explicit choice, else the output file's extension,
    /// else the stored default, else what the profile recommends. An output whose extension
    /// names another format is refused rather than written with a misleading name.
    pub fn resolve(
        explicit: Option<Format>,
        output: Option<&Path>,
        stored: Option<Format>,
        profile: &Profile,
    ) -> Result<Format> {
        let by_name = output.and_then(Format::of_path);
        match (explicit, by_name) {
            (Some(f), Some(n)) if f != n => Err(Error::Other(format!(
                "output {} is a .{} file but the format is {}",
                output.map(|p| p.display().to_string()).unwrap_or_default(),
                n.id(),
                f.id()
            ))),
            (Some(f), _) | (None, Some(f)) => Ok(f),
            (None, None) => Ok(stored.unwrap_or(profile.format)),
        }
    }
}

impl FromStr for Format {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Format::ALL
            .into_iter()
            .find(|f| f.id().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| Error::Other(format!("unknown format '{s}' (epub, azw3, cbz or pdf)")))
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}
