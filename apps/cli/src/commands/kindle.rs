use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{export, ExportRequest, DEVICES};
use mangas_core::library;

use super::cut_args::{print_progress, CutArgs};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Directories of downloaded chapters, in reading order
    pub chapters: Vec<PathBuf>,
    #[command(flatten)]
    pub cut: CutArgs,
    /// EPUB title (defaults to the first chapter directory's parent name)
    #[arg(long)]
    pub title: Option<String>,
    /// Output EPUB path [default: out.epub]
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    /// Print the known device profiles and exit
    #[arg(long)]
    pub list_devices: bool,
}

fn list_devices() -> anyhow::Result<()> {
    for d in DEVICES {
        outln!(
            "{:<22} {} ({}x{}, {} dpi)",
            d.id,
            d.name,
            d.width,
            d.height,
            d.dpi
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
    if args.list_devices {
        return list_devices();
    }
    if args.chapters.is_empty() {
        bail!("no chapter directories given");
    }
    let dev = args.cut.device()?;
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("out.epub"));
    let penalizer = args.cut.penalizer().await?;
    let req = ExportRequest {
        chapter_dirs: args.chapters.clone(),
        device: dev,
        cut: args.cut.options(),
        title: args
            .title
            .clone()
            .unwrap_or_else(|| default_title(&args.chapters)),
        output: output.clone(),
        meta: Default::default(),
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
/// Each folder is matched as made absolute and as resolved (symlinks), since the library may
/// store either spelling. The EPUB is already written, so a failed update only gets a warning.
pub(super) fn mark_exported(dirs: &[&Path]) {
    let mut spellings: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| [std::path::absolute(d).ok(), d.canonicalize().ok()])
        .flatten()
        .collect();
    spellings.sort();
    spellings.dedup();
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    let marked = paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| Ok(library::mark_exported(&lib, &dirs)?));
    if let Err(e) = marked {
        eprintln!("warning: could not mark the chapters as exported: {e}");
    }
}
