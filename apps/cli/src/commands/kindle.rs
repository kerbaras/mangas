//! `mangas kindle`: chapter folders to one EPUB, CBZ or PDF for an e-reader profile.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{export, BookMeta, BudgetSpec, ExportRequest};
use mangas_core::export::target::{book_path, is_folder};
use mangas_core::export::{ExportPreset, Format, PROFILES};
use mangas_core::library;
use mangas_core::storage;

use super::cut_args::{print_progress, CutArgs, DEFAULT_PROFILE};
use super::deliver::SendArg;
use super::deliver_usb;
use super::export_args::FormatArg;
use super::export_plan::Fewer;
use super::export_preset::note_series_preset;
use super::export_report::Report;
use super::export_size::{delivery_of, levers_text, warn_too_big};
use super::kindle_library::from_library;
use super::kindle_print;
use super::kindle_record;
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Directories of downloaded chapters, in reading order
    pub chapters: Vec<PathBuf>,
    #[command(flatten)]
    pub cut: CutArgs,
    /// File format [default: the output's extension, else epub]
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    /// Book title (defaults to the series in the library, else the first chapter directory's
    /// parent name)
    #[arg(long)]
    pub title: Option<String>,
    /// Output file, or a folder (existing, or ending in a separator) to write the book into
    /// under its title [default: out.<format>]
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    /// Write over the output file if it exists instead of next to it as "out (2)"
    #[arg(long)]
    pub replace: bool,
    /// After writing the book, email it to the Kindle or copy it to one over USB
    #[arg(long, value_enum)]
    pub send: Option<SendArg>,
    /// Mount point of the Kindle for `--send usb` [default: detected]
    #[arg(long)]
    pub kindle: Option<PathBuf>,
    /// With `--send usb`, copy a format the Kindle may not open (EPUB); CBZ is never copied
    #[arg(long, requires = "send")]
    pub force: bool,
    /// Print the known e-reader profiles and exit
    #[arg(long, visible_alias = "list-devices")]
    pub list_profiles: bool,
    /// Print one JSON object instead of the summary line: `files` (path, title, chapters,
    /// pages, skipped_files), `skipped` and `failed` (path, reason) and `detectors` (state
    /// "used", "skipped" or "off", and the runtime's error as reason)
    #[arg(long, conflicts_with = "list_profiles")]
    pub json: bool,
}

fn list_profiles() -> anyhow::Result<()> {
    for p in PROFILES {
        outln!(
            "{:<22} {} ({}x{}, {} dpi, {}, {})",
            p.id,
            p.name,
            p.width,
            p.height,
            p.dpi,
            if p.grayscale { "grey" } else { "colour" },
            p.format
        )?;
    }
    Ok(())
}

/// The library's series for the chapters, else the first folder's parent name unless that is
/// a manga id (a `<source>/<manga id>/<chapter>` download folder).
fn default_title(series: Option<&str>, chapters: &[PathBuf]) -> String {
    let folder =
        || library::series_folder_name(chapters.first()?, &super::kindle_titles::source_ids());
    series
        .map(str::to_owned)
        .or_else(folder)
        .unwrap_or_else(|| "Manga".to_string())
}

/// `--format`, else the output's extension, else the series preset's, else EPUB.
fn legacy_format(
    explicit: Option<Format>,
    output: Option<&Path>,
    series: Option<Format>,
) -> Format {
    explicit
        .or_else(|| output.and_then(Format::of_path))
        .or(series)
        .unwrap_or(Format::Epub)
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if args.list_profiles {
        return list_profiles();
    }
    if args.chapters.is_empty() {
        bail!("no chapter directories given");
    }
    let _reading = storage::hold_for_export(paths::data_dir)?;
    // Kept from before other formats existed: a Paperwhite EPUB unless the flags or the
    // series preset say otherwise, whatever the stored defaults are. The JPEG quality stored for
    // the profile applies: it belongs to the device, not to the default picks.
    let explicit = args.cut.explicit(None)?;
    let (labels, series, chapter_titles, saved, source) = from_library(&args.chapters);
    let preset = ExportPreset::resolve(&explicit, saved.as_ref(), &source)
        .with_stored_quality(&super::stored_settings(), DEFAULT_PROFILE);
    let dev = CutArgs::device_of(&preset, DEFAULT_PROFILE)?;
    let file = args.output.as_deref().filter(|o| !is_folder(o));
    let picked = legacy_format(
        args.format.map(Format::from),
        file,
        saved.as_ref().and_then(|p| p.format),
    );
    let format = Format::resolve(Some(picked), file, None, &dev)?;
    let title = args
        .title
        .clone()
        .unwrap_or_else(|| default_title(series.as_deref(), &args.chapters));
    let output = match &args.output {
        Some(o) => book_path(o, &title, format),
        None => PathBuf::from(format!("out.{format}")),
    };
    note_series_preset(saved.as_ref());
    let (penalizer, detectors) = args.cut.penalizer().await?;
    let req = ExportRequest {
        split: args.cut.split(),
        chapter_dirs: args.chapters.clone(),
        device: dev,
        format,
        cut: preset.cut(),
        page: preset.page(),
        title,
        title_explicit: args.title.is_some(),
        output: output.clone(),
        replace: args.replace,
        meta: BookMeta {
            chapter_titles,
            direction: preset.direction.unwrap_or_default(),
            ..Default::default()
        },
        labels,
    };
    deliver_usb::precheck(args.send, format.id(), args.kindle.as_deref(), args.force)?;
    let started = Instant::now();
    let mut report = Report::new(detectors);
    let stats = export(&req, penalizer, &mut print_progress)
        .or_else(|e| report.export_failed(&req.output, req.replace, e, args.json))?;
    let spec = BudgetSpec {
        device: dev,
        cut: req.cut,
        page: req.page,
        delivery: delivery_of(args.send),
        split: req.split,
    };
    let fix = || levers_text(None, &req.chapter_dirs, spec, Fewer::Folders);
    warn_too_big(&stats.oversized, spec.delivery, fix);
    report.add(&stats, &req.title, &req.chapter_dirs);
    if !args.json {
        kindle_print::print_book(&stats, req.chapter_dirs.len(), started)?;
    }
    let to = args
        .send
        .map(|via| (via, args.kindle.as_deref(), args.force));
    let done =
        kindle_record::record_and_send(&stats, &req.chapter_dirs, &req.title, dev.id, to).await;
    done.print_sent(args.json)?;
    if args.json {
        report.recorded.clone_from(&done.recorded);
        for (file, unsent) in &done.failed {
            report.unsent(file, unsent);
        }
        report.print()?;
    }
    done.result()
}

#[cfg(test)]
#[path = "kindle_tests.rs"]
mod tests;
