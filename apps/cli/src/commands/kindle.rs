//! `mangas kindle`: chapter folders to one EPUB, CBZ or PDF for an e-reader profile.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{export, ExportRequest};
use mangas_core::export::{Format, Labels, PROFILES};
use mangas_core::library;

use super::cut_args::{print_progress, CutArgs};
use super::export_args::FormatArg;
use super::stored_settings;
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Directories of downloaded chapters, in reading order
    pub chapters: Vec<PathBuf>,
    #[command(flatten)]
    pub cut: CutArgs,
    /// File format [default: the output's extension, the stored default, else what the profile recommends]
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    /// Book title (defaults to the first chapter directory's parent name)
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

fn default_title(chapters: &[PathBuf]) -> String {
    chapters
        .first()
        .and_then(|c| c.canonicalize().ok())
        .and_then(|c| {
            c.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Manga".to_string())
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if args.list_profiles {
        return list_profiles();
    }
    if args.chapters.is_empty() {
        bail!("no chapter directories given");
    }
    let dev = args.cut.device()?;
    let explicit = args.format.map(Format::from);
    let stored = stored_settings().default_format;
    let format = Format::resolve(explicit, args.output.as_deref(), stored, &dev)?;
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("out.{format}")));
    let penalizer = args.cut.penalizer().await?;
    let req = ExportRequest {
        chapter_dirs: args.chapters.clone(),
        device: dev,
        format,
        cut: args.cut.options(),
        title: args
            .title
            .clone()
            .unwrap_or_else(|| default_title(&args.chapters)),
        title_explicit: args.title.is_some(),
        output: output.clone(),
        meta: Default::default(),
        labels: labels_of(&args.chapters),
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

/// Series/chapter/volume labels from the library; folders it doesn't know export unlabelled.
fn labels_of(chapters: &[PathBuf]) -> Labels {
    let refs: Vec<&Path> = chapters.iter().map(PathBuf::as_path).collect();
    let spellings = spellings(&refs);
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| Ok(library::export_labels(&lib, &dirs, chapters.len())?))
        .unwrap_or_else(|e: anyhow::Error| {
            eprintln!("warning: could not read the chapters' series and numbers: {e}");
            Labels::default()
        })
}

/// Each folder made absolute and resolved (symlinks), since the library may store either spelling.
fn spellings(dirs: &[&Path]) -> Vec<PathBuf> {
    let mut spellings: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| [std::path::absolute(d).ok(), d.canonicalize().ok()])
        .flatten()
        .collect();
    spellings.sort();
    spellings.dedup();
    spellings
}
