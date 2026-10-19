//! Smart-cut preview: the first pages an export of one chapter would produce, written as JPEG
//! files so a UI can show them. Same [`Run`] as the export, stopped once enough pages are final.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::run::{Run, RunSpec};
use super::Device;
use crate::domain::{Error, Result};
use crate::export::PageSettings;
use crate::smartcut::{Options, RowPenalizer};

#[derive(Debug, Clone)]
pub struct PreviewRequest {
    pub chapter_dir: PathBuf,
    pub device: Device,
    pub cut: Options,
    pub page: PageSettings,
    /// At most this many pages are rendered (at least one).
    pub limit: usize,
    /// Created if missing; pages are written as `page-001.jpg`, `page-002.jpg`, ...
    pub out_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewPage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preview {
    pub pages: Vec<PreviewPage>,
    /// The chapter gives more pages than were rendered.
    pub more: bool,
    pub elapsed_ms: u128,
    /// Why detectors were asked for but not used (set by the caller that chose them).
    #[serde(default)]
    pub detectors_error: Option<String>,
}

fn write_page(dir: &Path, index: usize, jpeg: &[u8]) -> Result<PathBuf> {
    let path = dir.join(format!("page-{:03}.jpg", index + 1));
    std::fs::write(&path, jpeg).map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
    Ok(path)
}

/// Cuts the start of `req.chapter_dir` like an export would. `cancelled` is polled between
/// source images so a newer preview can abandon this one.
pub fn preview(
    req: &PreviewRequest,
    penalizer: Option<RowPenalizer>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Preview> {
    let t = Instant::now();
    let limit = req.limit.max(1);
    let mut ignore = |_| {};
    let spec = RunSpec {
        device: req.device,
        cut: req.cut,
        page: req.page,
        penalizer,
        limit: Some(limit),
        cancelled,
    };
    let mut run = Run::new(spec, &mut ignore);
    run.chapter(&req.chapter_dir)?;
    let more = run.stopped_early || run.pages.len() > limit;
    std::fs::create_dir_all(&req.out_dir)
        .map_err(|e| Error::Other(format!("{}: {e}", req.out_dir.display())))?;
    let pages = run
        .pages
        .iter()
        .take(limit)
        .enumerate()
        .map(|(i, p)| {
            write_page(&req.out_dir, i, &p.jpeg).map(|path| PreviewPage {
                path,
                width: p.width,
                height: p.height,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Preview {
        pages,
        more,
        elapsed_ms: t.elapsed().as_millis(),
        detectors_error: None,
    })
}
