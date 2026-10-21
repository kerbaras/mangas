//! Out of disk space while exporting: one plain, final error naming the folder, whatever the OS
//! called it (disk full, quota, file size limit) and whichever write hit it.

use std::io::ErrorKind;
use std::path::Path;

use crate::domain::unplaced;
use crate::domain::Error;

/// The disk, the user's quota or the process's file size limit can't take more bytes.
pub(crate) fn is_out_of_space(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded | ErrorKind::FileTooLarge
    )
}

/// [`Error::NoSpace`] for `folder`, named absolutely so the message says which disk.
pub(crate) fn no_space(folder: &Path) -> Error {
    Error::NoSpace {
        folder: std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf()),
    }
}

/// `e` as [`Error::NoSpace`] in `folder` when it is an out-of-space IO error, else unchanged.
pub(crate) fn in_folder(e: Error, folder: &Path) -> Error {
    match e {
        Error::Io { source, .. } if is_out_of_space(&source) => no_space(folder),
        e => e,
    }
}

/// A zip writer error: IO stays [`Error::Io`] so a full disk is recognised, the rest is labelled
/// with the format.
pub(crate) fn zip_error(format: &'static str) -> impl Fn(zip::result::ZipError) -> Error + Copy {
    move |e| match e {
        zip::result::ZipError::Io(io) => unplaced(io),
        e => Error::Other(format!("{format}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io(kind: ErrorKind) -> Error {
        unplaced(std::io::Error::from(kind))
    }

    #[test]
    fn full_disk_quota_and_size_limit_read_as_no_space() {
        let dir = Path::new("/media/kindle");
        for kind in [
            ErrorKind::StorageFull,
            ErrorKind::QuotaExceeded,
            ErrorKind::FileTooLarge,
        ] {
            let e = in_folder(io(kind), dir);
            assert_eq!(e.to_string(), "not enough space in /media/kindle");
        }
    }

    #[test]
    fn other_errors_pass_through() {
        let e = in_folder(io(ErrorKind::PermissionDenied), Path::new("/x"));
        assert!(matches!(e, Error::Io { .. }), "{e}");
        let zip = zip_error("epub")(zip::result::ZipError::Io(ErrorKind::StorageFull.into()));
        assert!(matches!(
            in_folder(zip, Path::new("/x")),
            Error::NoSpace { .. }
        ));
    }
}
