//! What a [`Run`](super::run::Run) is built from, and the stats it starts with.

use std::path::{Path, PathBuf};

use super::request::ExportStats;
use super::Device;
use crate::export::PageSettings;
use crate::smartcut::{Options, RowPenalizer};

pub(super) struct RunSpec<'a> {
    pub device: Device,
    pub cut: Options,
    pub page: PageSettings,
    pub penalizer: Option<RowPenalizer>,
    pub limit: Option<usize>,
    pub cancelled: &'a dyn Fn() -> bool,
    pub spool_dir: Option<&'a Path>,
}

pub(super) fn empty_stats() -> ExportStats {
    ExportStats {
        pages: 0,
        strips: 0,
        source_rows: 0,
        pagination_ms: 0,
        filters_ms: 0,
        skipped: 0,
        empty_chapters: Vec::new(),
        output: PathBuf::new(),
        parts: Vec::new(),
        detectors_error: None,
        oversized: Vec::new(),
        jpeg_quality: 0,
        chapters: Vec::new(),
    }
}
