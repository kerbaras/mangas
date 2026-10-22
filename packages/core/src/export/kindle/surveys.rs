//! Every chapter's [`Survey`], kept for the process: a plan, its levers (at several qualities
//! and book sizes) and the warning after a write all ask about the same chapters, and cutting
//! the strips is the slow part of an estimate. A survey is keyed by the chapter's files (path,
//! size, modification time) and every setting that changes the pages, so a chapter that changed
//! is surveyed again; one without the asked quality is surveyed again with it added.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use super::budget::QUALITY_STEPS;
use super::input::{chapter_files, ChapterFiles};
use super::run::{Run, RunSpec};
use super::survey::Survey;
use super::Device;
use crate::domain::Result;
use crate::export::PageSettings;
use crate::smartcut::Options;

/// Surveys kept before the cache starts over.
const KEPT: usize = 1024;

static SURVEYS: LazyLock<Mutex<HashMap<String, Arc<Survey>>>> = LazyLock::new(Default::default);

/// The survey of chapter `dir` on `device` with `cut` and `page` (strip pages encoded at its
/// quality, sampled at the plan's lower ones), without the detectors (a plan does not load
/// them). A folder that can't be listed has no pages.
pub(super) fn chapter_survey(
    dir: &Path,
    device: Device,
    cut: Options,
    page: PageSettings,
) -> Result<Arc<Survey>> {
    let Ok(files) = chapter_files(dir) else {
        return Ok(Arc::new(Survey::new(0, Vec::new())));
    };
    let quality = page.image_settings(&device).quality;
    let key = key(dir, &files.pages, device, cut, page);
    let cached = SURVEYS.lock().ok().and_then(|m| m.get(&key).cloned());
    if let Some(s) = cached.as_ref().filter(|s| s.sampled(quality)) {
        return Ok(s.clone());
    }
    let mut qualities: Vec<u8> = (QUALITY_STEPS.into_iter())
        .chain(
            cached
                .iter()
                .flat_map(|s| s.qualities.iter().copied().chain([s.base])),
        )
        .collect();
    qualities.sort_unstable_by(|a, b| b.cmp(a));
    qualities.dedup();
    let survey = Arc::new(survey(dir, files, device, cut, page, qualities)?);
    if let Ok(mut m) = SURVEYS.lock() {
        if m.len() >= KEPT {
            m.clear();
        }
        m.insert(key, survey.clone());
    }
    Ok(survey)
}

fn survey(
    dir: &Path,
    files: ChapterFiles,
    device: Device,
    cut: Options,
    page: PageSettings,
    qualities: Vec<u8>,
) -> Result<Survey> {
    let mut ignore = |_| {};
    let spec = RunSpec {
        device,
        cut,
        page,
        penalizer: None,
        limit: None,
        cancelled: &|| false,
        spool_dir: None,
    };
    let mut run = Run::new(spec, &mut ignore)?;
    run.survey = Some(Survey::new(run.settings.quality, qualities));
    run.files(dir, files)?;
    Ok((run.survey.take()).unwrap_or_else(|| Survey::new(0, Vec::new())))
}

fn key(dir: &Path, files: &[PathBuf], device: Device, cut: Options, page: PageSettings) -> String {
    let stamps: Vec<_> = (files.iter())
        .map(|f| {
            std::fs::metadata(f)
                .map(|m| (m.len(), m.modified().ok()))
                .ok()
        })
        .collect();
    let page = PageSettings {
        quality: None,
        ..page
    };
    format!("{dir:?}|{files:?}|{stamps:?}|{device:?}|{cut:?}|{page:?}")
}
