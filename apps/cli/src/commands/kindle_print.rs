//! The plain-text summary `mangas kindle` prints for the book it wrote.

use std::time::Instant;

use mangas_core::export::kindle::{part_book, ExportStats};

use crate::output::outln;

pub fn print_book(stats: &ExportStats, chapters: usize, started: Instant) -> std::io::Result<()> {
    let (book, parts) = match stats.parts.len() {
        0 => (stats.output.clone(), String::new()),
        n => (part_book(&stats.output), format!(" in {n} parts")),
    };
    outln!(
        "{}: {} pages{parts} from {} chapters ({} strips, {} source rows); pagination {} ms, filters {} ms, total {} ms",
        book.display(),
        stats.pages,
        chapters,
        stats.strips,
        stats.source_rows,
        stats.pagination_ms,
        stats.filters_ms,
        started.elapsed().as_millis()
    )?;
    for part in &stats.parts {
        outln!("  {}: {} pages", part.path.display(), part.pages)?;
    }
    if stats.skipped > 0 {
        outln!(
            "skipped {} unsupported file(s), listed in the warnings above",
            stats.skipped
        )?;
    }
    Ok(())
}
