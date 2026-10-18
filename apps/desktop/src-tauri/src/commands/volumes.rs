//! Volume export: plan the books of a manga's downloaded chapters, then write each one.

use std::path::PathBuf;

use mangas_core::export::kindle::{self, ExportStats, Progress};
use mangas_core::export::volume::{self, Grouping, VolumePlan};
use mangas_core::smartcut::Preset;
use tauri::AppHandle;

use super::covers::cover_file;
use super::kindle::{emit, mark_exported, penalizer, ExportEvent};
use super::{open_library, to_ipc};

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

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn export_volumes(
    app: AppHandle,
    source: String,
    manga_id: String,
    chapter_ids: Vec<String>,
    grouping: Grouping,
    title: Option<String>,
    device: String,
    preset: Preset,
    detectors: bool,
    output_dir: PathBuf,
) -> Result<Vec<ExportStats>, String> {
    let dev =
        kindle::device(&device).ok_or_else(|| to_ipc(format!("unknown device '{device}'")))?;
    let (src, plans) = plans(&source, &manga_id, &chapter_ids, grouping, title.as_deref())?;
    if plans.is_empty() {
        return Err(to_ipc("none of these chapters is downloaded"));
    }
    let penalizer = penalizer(&app, detectors).await?;
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
            preset.options(),
            &output_dir,
            cover.clone(),
        );
        let (handle, penalizer) = (app.clone(), penalizer.clone());
        let (stats, dirs) = tauri::async_runtime::spawn_blocking(move || {
            let mut on_progress = |p: Progress| emit(&handle, ExportEvent::Export(p));
            kindle::export(&req, penalizer, &mut on_progress).map(|s| (s, req.chapter_dirs))
        })
        .await
        .map_err(to_ipc)?
        .map_err(to_ipc)?;
        mark_exported(&stats.exported_chapters(&dirs));
        written.push(stats);
    }
    Ok(written)
}
