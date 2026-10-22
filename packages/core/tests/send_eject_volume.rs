//! Eject only ever unmounts a volume that is the Kindle itself, never the drive a folder sits on.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use mangas_core::send::eject::{eject_with, own_volume, Backend, StepError, Strategy};
use mangas_core::send::KindleVolume;

/// A backend whose only mount is `/`; records what it was asked to do.
#[derive(Default)]
struct OnSlash(RefCell<Vec<String>>);

impl Backend for OnSlash {
    fn mount_point(&self, _: &Path) -> Option<PathBuf> {
        Some("/".into())
    }
    fn flush(&self, _: &[PathBuf]) -> std::io::Result<()> {
        self.0.borrow_mut().push("flush".into());
        Ok(())
    }
    fn run(&self, strategy: Strategy, _: &Path) -> Result<bool, StepError> {
        self.0.borrow_mut().push(format!("{strategy:?}"));
        Ok(true)
    }
    fn holders(&self, _: &Path) -> Vec<String> {
        Vec::new()
    }
}

#[test]
fn a_kindle_folder_on_the_system_drive_is_never_unmounted() {
    let folder = KindleVolume {
        name: "Kindle".into(),
        root: "/home/me/Kindle".into(),
        documents: "/home/me/Kindle/documents".into(),
        ejectable: true,
    };
    let backend = OnSlash::default();
    let err = eject_with(&backend, &folder, &[], None).unwrap_err();
    assert!(
        err.to_string().contains("isn't a drive of its own"),
        "{err}"
    );
    assert!(err.to_string().contains("still there"), "{err}");
    assert!(backend.0.borrow().is_empty(), "{:?}", backend.0.borrow());
}

#[test]
fn own_volume_matches_the_mount_itself_only() {
    assert!(own_volume(
        Path::new("/media/me/KINDLE"),
        Path::new("/media/me/KINDLE/")
    ));
    assert!(!own_volume(
        Path::new("/media/me"),
        Path::new("/media/me/KINDLE")
    ));
    assert!(!own_volume(Path::new("/"), Path::new("/")));
}
