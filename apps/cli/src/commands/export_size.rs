//! The Send-to-Kindle size check after a book is written: a file over the email limit is named
//! on stderr with the flags that make it fit.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::{ExportStats, Oversized, PART_SIZE, SEND_TO_KINDLE_LIMIT};

/// Size of `file` in bytes; 0 when it can't be read.
pub fn bytes(file: &Path) -> u64 {
    std::fs::metadata(file).map(|m| m.len()).unwrap_or(0)
}

/// Warns about each written file over the Send-to-Kindle email limit (measured on disk by core).
pub fn warn_too_big(files: &[Oversized]) {
    for file in files {
        if let Some(line) = too_big(&file.path, file.bytes) {
            eprintln!("{line}");
        }
    }
}

/// The warning for `file` of `size` bytes, when it is over the limit.
pub fn too_big(file: &Path, size: u64) -> Option<String> {
    (size > SEND_TO_KINDLE_LIMIT).then(|| {
        format!(
            "warning: {} is {:.1} MB, over the {} MB Send-to-Kindle email limit; write it in \
             parts with --split-mb {} or make it smaller with --quality 70",
            file.display(),
            size as f64 / 1e6,
            SEND_TO_KINDLE_LIMIT / 1_000_000,
            PART_SIZE / 1_000_000
        )
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_file_over_the_email_limit_gets_a_warning() {
        let file = Path::new("/k/Big.epub");
        assert_eq!(too_big(file, 50_000_000), None);
        assert_eq!(
            too_big(file, 96_400_000).as_deref(),
            Some(
                "warning: /k/Big.epub is 96.4 MB, over the 50 MB Send-to-Kindle email limit; \
                 write it in parts with --split-mb 45 or make it smaller with --quality 70"
            )
        );
    }
}
