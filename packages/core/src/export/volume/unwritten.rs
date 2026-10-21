//! The file names the books a run did not write would have taken. Without `replace`, a book takes
//! the first free ` (n)` name at write time, so reporting the planned name could point at a book
//! already in the folder that the run never touched.

use std::path::{Path, PathBuf};

use crate::export::free_names_in;
use crate::export::kindle::ExportRequest;

/// Each request's file name as the writer would pick it now, in order: later books also skip the
/// names taken by the ones before them. A folder that cannot be listed is asked name by name.
pub(super) fn unwritten_paths(reqs: &[ExportRequest]) -> Vec<PathBuf> {
    reqs.iter()
        .enumerate()
        .map(|(i, r)| match r.replace {
            true => r.output.clone(),
            false => free_path(&reqs[..=i], r).unwrap_or_else(|| r.output.clone()),
        })
        .collect()
}

fn free_path(upto: &[ExportRequest], r: &ExportRequest) -> Option<PathBuf> {
    let parent = r.output.parent();
    let wanted: Vec<String> = upto
        .iter()
        .filter(|o| !o.replace && o.output.parent() == parent)
        .filter_map(|o| Some(o.output.file_name()?.to_string_lossy().into_owned()))
        .collect();
    let dir = parent.filter(|d| !d.as_os_str().is_empty());
    let names = free_names_in(dir.unwrap_or(Path::new(".")), &wanted).ok()?;
    Some(r.output.with_file_name(names.last()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(dir: &Path, name: &str, replace: bool) -> ExportRequest {
        ExportRequest {
            chapter_dirs: Vec::new(),
            device: crate::export::profile("kindle-paperwhite12").unwrap(),
            format: crate::export::Format::Epub,
            cut: crate::smartcut::Preset::Balanced.options(),
            page: Default::default(),
            title: name.into(),
            title_explicit: false,
            output: dir.join(format!("{name}.epub")),
            replace,
            split: None,
            meta: Default::default(),
            labels: Default::default(),
        }
    }

    #[test]
    fn names_already_in_the_folder_get_the_numbered_name_the_writer_takes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("v2.epub"), b"old").unwrap();
        std::fs::write(dir.path().join("v3.epub"), b"old").unwrap();
        let reqs = [req(dir.path(), "v2", false), req(dir.path(), "v3", false)];
        assert_eq!(
            unwritten_paths(&reqs),
            [
                dir.path().join("v2 (2).epub"),
                dir.path().join("v3 (2).epub")
            ]
        );
    }

    #[test]
    fn replacing_books_keep_their_planned_names() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("v2.epub"), b"old").unwrap();
        let reqs = [req(dir.path(), "v2", true)];
        assert_eq!(unwritten_paths(&reqs), [dir.path().join("v2.epub")]);
    }
}
