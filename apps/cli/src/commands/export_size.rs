//! The Send-to-Kindle size check after a book is written: a file over its delivery's budget
//! (50 MB by email, 200 MB otherwise) is named on stderr with the flags that make it fit.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::{Delivery, ExportStats, Oversized};

use super::deliver::SendArg;

use crate::output::outln;

/// Size of `file` in bytes; 0 when it can't be read.
pub fn bytes(file: &Path) -> u64 {
    std::fs::metadata(file).map(|m| m.len()).unwrap_or(0)
}

/// The budget `send` puts a book under: email's, or the web one for everything else.
pub fn delivery_of(send: Option<SendArg>) -> Delivery {
    match send {
        Some(SendArg::Email) => Delivery::Email,
        _ => Delivery::Other,
    }
}

/// Warns about each written file over `delivery`'s budget (measured on disk by core).
pub fn warn_too_big(files: &[Oversized], delivery: Delivery) {
    for file in delivery.over(files) {
        eprintln!("{}", too_big(&file.path, file.bytes, delivery));
    }
}

/// The warning for `file` of `size` bytes over `delivery`'s budget.
pub fn too_big(file: &Path, size: u64, delivery: Delivery) -> String {
    format!(
        "warning: {} is {:.1} MB, over the {} MB Send-to-Kindle {} limit; write it in parts \
         with --split-mb {} or make it smaller with --quality 70",
        file.display(),
        size as f64 / 1e6,
        delivery.budget() / 1_000_000,
        delivery.limit_name(),
        delivery.target() / 1_000_000
    )
}

/// Each file written and its page count: the parts, else the one book.
pub fn written_pages(stats: &ExportStats) -> Vec<(PathBuf, usize)> {
    match stats.parts.is_empty() {
        true => vec![(stats.output.clone(), stats.pages)],
        false => stats
            .parts
            .iter()
            .map(|p| (p.path.clone(), p.pages))
            .collect(),
    }
}

/// The files a book was written to, with their pages, and the files it skipped.
pub fn print_written(stats: &ExportStats) -> std::io::Result<()> {
    for (file, pages) in written_pages(stats) {
        outln!("{}: {pages} pages", file.display())?;
    }
    if stats.skipped > 0 {
        outln!(
            "skipped {} unsupported file(s), listed in the warnings above",
            stats.skipped
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_warned_about_against_the_budget_of_its_delivery() {
        let file = Path::new("/k/Big.epub");
        let big = |bytes| Oversized {
            path: file.to_path_buf(),
            bytes,
        };
        let files = [big(96_400_000), big(212_000_000)];
        assert_eq!(Delivery::Email.over(&files).len(), 2);
        assert_eq!(Delivery::Other.over(&files), [big(212_000_000)]);
        assert_eq!(
            too_big(file, 96_400_000, Delivery::Email),
            "warning: /k/Big.epub is 96.4 MB, over the 50 MB Send-to-Kindle email limit; \
             write it in parts with --split-mb 45 or make it smaller with --quality 70"
        );
        assert_eq!(
            too_big(file, 212_000_000, Delivery::Other),
            "warning: /k/Big.epub is 212.0 MB, over the 200 MB Send-to-Kindle web limit; \
             write it in parts with --split-mb 180 or make it smaller with --quality 70"
        );
    }
}
