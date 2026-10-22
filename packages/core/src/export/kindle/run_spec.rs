//! Building a [`Run`]: what it is made from, and the state it starts in.

use std::path::{Path, PathBuf};

use super::request::{ExportStats, Progress};
use super::run::Run;
use super::Device;
use crate::domain::Result;
use crate::export::{PageSettings, PageSpool};
use crate::smartcut::{Options, RowPenalizer, StripPaginator};

pub(super) struct RunSpec<'a> {
    pub device: Device,
    pub cut: Options,
    pub page: PageSettings,
    pub penalizer: Option<RowPenalizer>,
    pub limit: Option<usize>,
    pub cancelled: &'a dyn Fn() -> bool,
    pub spool_dir: Option<&'a Path>,
}

fn empty_stats() -> ExportStats {
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

impl<'a> Run<'a> {
    pub fn new(spec: RunSpec<'a>, progress: &'a mut dyn FnMut(Progress)) -> Result<Run<'a>> {
        let dev = spec.device;
        let mut paginator = StripPaginator::new(dev.width, spec.page.page_height(&dev), spec.cut);
        if let Some(p) = spec.penalizer {
            paginator = paginator.with_penalizer(p);
        }
        Ok(Run {
            settings: spec.page.image_settings(&dev),
            paginator,
            pages: PageSpool::new_in(spec.spool_dir)?,
            stats: empty_stats(),
            progress,
            device: dev,
            trim: spec.page.trim_margins,
            limit: spec.limit,
            cancelled: spec.cancelled,
            stopped_early: false,
            survey: None,
        })
    }
}
