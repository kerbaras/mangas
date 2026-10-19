//! Volume export: plan the books of a manga's downloaded chapters, then write each one.

use std::path::PathBuf;

use mangas_core::export::kindle::{self, ExportStats, Progress};
use mangas_core::export::volume::{self, Grouping, VolumePlan};
use mangas_core::export::{ExportPreset, Format};
use mangas_core::library;
use mangas_core::smartcut::Preset;
use tauri::{AppHandle, Runtime};

use super::covers::cover_file;
use super::export::{emit, record_export, resolve, ExportEvent};
use super::penalizer::penalizer;
use super::{open_library, settings_or_default, settings_quietly, to_ipc};
use crate::paths;

fn plans(
    source: &str,
    manga_id: &str,
    chapter_ids: &[String],
    grouping: Grouping,
    title: Option<&str>,
) -> Result<(volume::VolumeSource, Vec<VolumePlan>), String> {
    let lib = open_library()?;
    let src = volume::load(&lib, source, manga_id, Some(chapter_ids)).map_err(to_ipc)?;
    let plans = volume::plan(&src.manga, &src.chapters, grouping, None, title);
    Ok((src, plans))
}

/// The files an export would write, in order; nothing is read from disk but the library.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn plan_volumes(
    source: String,
    manga_id: String,
    chapter_ids: Vec<String>,
    grouping: Grouping,
    title: Option<String>,
) -> Result<Vec<VolumePlan>, String> {
    Ok(plans(&source, &manga_id, &chapter_ids, grouping, title.as_deref())?.1)
}

/// The names `names` would be written under in `dir` now (` (2)`, ` (3)`… for taken ones), so
/// the export dialog previews the files the export really writes.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn free_export_names(dir: PathBuf, names: Vec<String>) -> Result<Vec<String>, String> {
    let dir = mangas_core::export::in_export_folder(&paths::exports(&settings_quietly())?, &dir);
    mangas_core::export::free_names_in(&dir, &names).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
#[allow(clippy::too_many_arguments)]
/// A relative `output_dir`, or none, goes under the default export folder ([`paths::exports`]).
pub async fn export_volumes<R: Runtime>(
    app: AppHandle<R>,
    source: String,
    manga_id: String,
    chapter_ids: Vec<String>,
    grouping: Grouping,
    title: Option<String>,
    device: Option<String>,
    format: Option<Format>,
    preset: Option<Preset>,
    detectors: bool,
    output_dir: Option<PathBuf>,
    replace: Option<bool>,
) -> Result<Vec<ExportStats>, String> {
    let settings = settings_or_default();
    let output_dir = mangas_core::export::in_export_folder(
        &paths::exports(&settings)?,
        &output_dir.unwrap_or_default(),
    );
    let explicit = ExportPreset {
        cut_preset: preset,
        format,
        profile: device,
        ..Default::default()
    };
    let series =
        library::get_export_preset(&open_library()?, &source, &manga_id).map_err(to_ipc)?;
    let resolved = resolve(&explicit, series.as_ref(), &settings);
    let dev = resolved
        .profile_or(paths::DEFAULT_PROFILE)
        .map_err(to_ipc)?;
    let device = dev.id.to_string();
    let format = Format::resolve(None, None, resolved.format, &dev).map_err(to_ipc)?;
    let (src, plans) = plans(&source, &manga_id, &chapter_ids, grouping, title.as_deref())?;
    if plans.is_empty() {
        return Err(to_ipc("none of these chapters is downloaded"));
    }
    let detection = penalizer(&app, detectors, &settings).await?;
    let cover = cover_file(&src.manga).await;
    let mut written = Vec::with_capacity(plans.len());
    for (index, plan) in plans.iter().enumerate() {
        let title = plan.title.clone();
        emit(
            &app,
            ExportEvent::Volume {
                index,
                total: plans.len(),
                title,
            },
        );
        let req = volume::request(
            &src.manga,
            plan,
            dev,
            resolved.cut(),
            &output_dir,
            cover.clone(),
        )
        .with_format(format)
        .with_page(resolved.page())
        .with_replace(replace.unwrap_or(false));
        let (handle, penalizer) = (app.clone(), detection.penalizer.clone());
        let (mut stats, dirs) = tauri::async_runtime::spawn_blocking(move || {
            let mut on_progress = |p: Progress| emit(&handle, ExportEvent::Export(p));
            kindle::export(&req, penalizer, &mut on_progress).map(|s| (s, req.chapter_dirs))
        })
        .await
        .map_err(to_ipc)?
        .map_err(to_ipc)?;
        record_export(&stats, &dirs, &plan.title, &device);
        stats.detectors_error = detection.error.clone();
        written.push(stats);
    }
    Ok(written)
}
