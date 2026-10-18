//! `mangas kindle`: chapter folders to one EPUB, CBZ or PDF for an e-reader profile.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{export, ExportRequest};
use mangas_core::export::{Format, Labels, PROFILES};
use mangas_core::library::{self, Library};

use super::cut_args::{print_progress, CutArgs, DEFAULT_PROFILE};
use super::export_args::FormatArg;
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
    /// Output file [default: out.<format>]
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    /// Print the known e-reader profiles and exit
    #[arg(long, visible_alias = "list-devices")]
    pub list_profiles: bool,
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

/// `--format`, else the output's extension, else EPUB.
fn legacy_format(explicit: Option<Format>, output: Option<&Path>) -> Format {
    explicit
        .or_else(|| output.and_then(Format::of_path))
        .unwrap_or(Format::Epub)
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if args.list_profiles {
        return list_profiles();
    }
    if args.chapters.is_empty() {
        bail!("no chapter directories given");
    }
    // Kept from before other formats existed: a Paperwhite EPUB unless the flags say
    // otherwise, whatever the stored defaults are.
    let dev = args.cut.device_or(DEFAULT_PROFILE)?;
    let explicit = legacy_format(args.format.map(Format::from), args.output.as_deref());
    let format = Format::resolve(Some(explicit), args.output.as_deref(), None, &dev)?;
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("out.{format}")));
    let penalizer = args.cut.penalizer().await?;
    let (labels, series) = from_library(&args.chapters);
    let req = ExportRequest {
        chapter_dirs: args.chapters.clone(),
        device: dev,
        format,
        cut: args.cut.options(),
        title: args
            .title
            .clone()
            .unwrap_or_else(|| default_title(series.as_deref(), &args.chapters)),
        title_explicit: args.title.is_some(),
        output: output.clone(),
        meta: Default::default(),
        labels,
    };
    let started = Instant::now();
    let stats = export(&req, penalizer, &mut print_progress)?;
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
    if stats.skipped > 0 {
        outln!(
            "skipped {} unsupported file(s), listed in the warnings above",
            stats.skipped
        )?;
    }
    mark_exported(&stats.exported_chapters(&req.chapter_dirs));
    Ok(())
}

/// Library chapters among `dirs` count as read from now on; folders from elsewhere are ignored.
/// The file is already written, so a failed update only gets a warning.
pub(super) fn mark_exported(dirs: &[&Path]) {
    let spellings = spellings(dirs);
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    let marked = paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| Ok(library::mark_exported(&lib, &dirs)?));
    if let Err(e) = marked {
        eprintln!("warning: could not mark the chapters as exported: {e}");
    }
}

/// Series/chapter/volume labels from the library (folders it doesn't know export unlabelled),
/// plus the first chapter's series for the default title, which a mixed export lacks in `Labels`.
fn from_library(chapters: &[PathBuf]) -> (Labels, Option<String>) {
    paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| Ok((labels_in(&lib, chapters)?, first_series(&lib, chapters)?)))
        .unwrap_or_else(|e: anyhow::Error| {
            eprintln!("warning: could not read the chapters' series and numbers: {e}");
            (Labels::default(), None)
        })
}

fn first_series(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Option<String>> {
    Ok(labels_in(lib, &chapters[..chapters.len().min(1)])?.series)
}

fn labels_in(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Labels> {
    let refs: Vec<&Path> = chapters.iter().map(PathBuf::as_path).collect();
    let spellings = spellings(&refs);
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    Ok(library::export_labels(lib, &dirs, chapters.len())?)
}

/// Each folder as given, made absolute and resolved (symlinks): the library may store any of them.
fn spellings(dirs: &[&Path]) -> Vec<PathBuf> {
    let mut spellings: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| {
            [
                Some(d.to_path_buf()),
                std::path::absolute(d).ok(),
                d.canonicalize().ok(),
            ]
        })
        .flatten()
        .collect();
    spellings.sort();
    spellings.dedup();
    spellings
}

#[cfg(test)]
#[path = "kindle_tests.rs"]
mod tests;
