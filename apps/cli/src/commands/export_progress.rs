//! One export with the terminal progress printer.

use mangas_core::domain::Result;
use mangas_core::export::kindle::{export, ExportRequest, ExportStats, Progress};
use mangas_core::smartcut::RowPenalizer;

use super::cut_args::print_progress;

/// Runs `req`, printing progress. A failure mid-chapter starts its own line, not after the
/// `\r  pages: N` counter.
pub fn export_printing(
    req: &ExportRequest,
    penalizer: Option<RowPenalizer>,
) -> Result<ExportStats> {
    let mut mid_line = false;
    let mut progress = |p: Progress| {
        mid_line = matches!(p, Progress::Pages { .. });
        print_progress(p)
    };
    let exported = export(req, penalizer, &mut progress);
    if exported.is_err() && mid_line {
        eprintln!();
    }
    exported
}
