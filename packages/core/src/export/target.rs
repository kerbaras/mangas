//! Where a single book goes when the user names an output path: a file, or a folder to write
//! the book into under its planned name.

use std::path::{is_separator, Path, PathBuf};

use super::volume::file_name;
use super::Format;

/// Whether `output` names a folder: an existing directory, or a path ending in a separator.
pub fn is_folder(output: &Path) -> bool {
    output.is_dir() || output.as_os_str().to_string_lossy().ends_with(is_separator)
}

/// The file a book titled `title` is written to for the output path `output`: inside it under
/// its planned name ([`file_name`] as `format`) when it names a folder ([`is_folder`]), else
/// `output`.
pub fn book_path(output: &Path, title: &str, format: Format) -> PathBuf {
    match is_folder(output) {
        true => output.join(file_name(title)).with_extension(format.id()),
        false => output.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_gets_the_planned_name_and_a_file_path_stays() {
        let dir = tempfile::tempdir().unwrap();
        let at = |p: &Path| book_path(p, "Re:Zero v1", Format::Cbz);
        assert_eq!(at(dir.path()), dir.path().join("Re Zero v1.cbz"));
        let new = dir.path().join("new").join("");
        assert_eq!(at(&new), dir.path().join("new/Re Zero v1.cbz"));
        let file = dir.path().join("book.epub");
        assert_eq!(at(&file), file);
        std::fs::write(&file, "x").unwrap();
        assert_eq!(at(&file), file);
    }
}
