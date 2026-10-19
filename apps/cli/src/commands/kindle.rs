//! `mangas kindle`: chapter folders to one EPUB, CBZ or PDF for an e-reader profile.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{export, BookMeta, ExportRequest};
use mangas_core::export::{ExportPreset, Format, PROFILES};

use super::cut_args::{print_progress, CutArgs, DEFAULT_PROFILE};
use super::deliver::SendArg;
use super::export_args::FormatArg;
use super::export_preset::note_series_preset;
use super::export_report::Report;
use super::kindle_library::from_library;
use super::kindle_record;
use crate::output::outln;

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
    /// Output file [default: out.<format>]
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

/// The library's series for the chapters, else the first folder's parent name.
fn default_title(series: Option<&str>, chapters: &[PathBuf]) -> String {
    let folder = || {
        let dir = chapters.first()?.canonicalize().ok()?;
        Some(dir.parent()?.file_name()?.to_string_lossy().into_owned())
    };
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
    // Kept from before other formats existed: a Paperwhite EPUB unless the flags or the
    // series preset say otherwise, whatever the stored defaults are.
    let explicit = args.cut.explicit(None)?;
    let (labels, series, chapter_titles, saved) = from_library(&args.chapters);
    let preset = ExportPreset::resolve(&explicit, saved.as_ref(), &ExportPreset::default());
    let dev = CutArgs::device_of(&preset, DEFAULT_PROFILE)?;
    let picked = legacy_format(
        args.format.map(Format::from),
        args.output.as_deref(),
        saved.as_ref().and_then(|p| p.format),
    );
    let format = Format::resolve(Some(picked), args.output.as_deref(), None, &dev)?;
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("out.{format}")));
    note_series_preset(saved.as_ref());
    let (penalizer, detectors) = args.cut.penalizer().await?;
    let req = ExportRequest {
        chapter_dirs: args.chapters.clone(),
        device: dev,
        format,
        cut: preset.cut(),
        page: preset.page(),
        title: args
            .title
            .clone()
            .unwrap_or_else(|| default_title(series.as_deref(), &args.chapters)),
        title_explicit: args.title.is_some(),
        output: output.clone(),
        replace: args.replace,
        meta: BookMeta {
            chapter_titles,
            ..Default::default()
        },
        labels,
    };
    let started = Instant::now();
    let stats = export(&req, penalizer, &mut print_progress)?;
    let mut report = Report::new(detectors);
    report.add(&stats, &req.title, &req.chapter_dirs);
    if !args.json {
        outln!(
            "{}: {} pages from {} chapters ({} strips, {} source rows); pagination {} ms, filters {} ms, total {} ms",
            stats.output.display(),
            stats.pages,
            req.chapter_dirs.len(),
            stats.strips,
            stats.source_rows,
            stats.pagination_ms,
            stats.filters_ms,
            started.elapsed().as_millis()
        )?;
    }
    if stats.skipped > 0 && !args.json {
        outln!(
            "skipped {} unsupported file(s), listed in the warnings above",
            stats.skipped
        )?;
    }
    let to = args.send.map(|via| (via, args.kindle.as_deref()));
    let done =
        kindle_record::record_and_send(&stats, &req.chapter_dirs, &req.title, dev.id, to).await;
    match (&done, args.json) {
        (Ok(Some(sent)), false) => outln!("{sent}")?,
        (Ok(Some(sent)), true) => eprintln!("{sent}"),
        (Err(e), true) => report.fail(&stats.output, e.to_string()),
        _ => {}
    }
    if args.json {
        report.print()?;
    }
    done.map(|_| ())
}

#[cfg(test)]
#[path = "kindle_tests.rs"]
mod tests;
