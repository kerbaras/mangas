//! Finding mounted Kindles. A volume counts when it has a `documents/` folder and either is
//! named like a Kindle or also has the Kindle's `system/` folder.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// A Kindle the app can copy to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindleVolume {
    /// Name shown to the user (the volume or MTP device name).
    pub name: String,
    /// Mount point of the volume.
    pub root: PathBuf,
    /// Where books go.
    pub documents: PathBuf,
    /// A drive the OS can eject; `false` over MTP (a gvfs folder, nothing mounted).
    pub ejectable: bool,
}

/// Decides from the volume's path (its last two components are searched for "kindle") and the
/// folders found in it.
pub fn looks_like_kindle(path: &Path, has_documents: bool, has_system: bool) -> bool {
    let named = path.components().rev().take(2).any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .to_lowercase()
            .contains("kindle")
    });
    has_documents && (named || has_system)
}

/// Where this OS mounts removable volumes, each with how deep below it volumes sit
/// (0: the path itself is a volume).
pub fn mount_roots() -> Vec<(PathBuf, usize)> {
    let mut roots: Vec<(PathBuf, usize)> = Vec::new();
    if cfg!(target_os = "macos") {
        roots.push(("/Volumes".into(), 1));
    } else if cfg!(windows) {
        roots.extend((b'D'..=b'Z').map(|l| (PathBuf::from(format!("{}:\\", l as char)), 0)));
    } else {
        // /media/<user>/<volume> on Ubuntu, /media/<volume> elsewhere; gvfs MTP mounts are
        // /run/user/<uid>/gvfs/mtp:host=<device>/<storage>.
        roots.extend([
            ("/media".into(), 2),
            ("/run/media".into(), 2),
            ("/mnt".into(), 1),
            ("/run/user".into(), 4),
        ]);
    }
    roots
}

/// Kindles mounted right now.
pub fn find_kindles() -> Vec<KindleVolume> {
    find_kindles_in(&mount_roots())
}

/// Kindles found within `depth` levels below each root.
pub fn find_kindles_in(roots: &[(PathBuf, usize)]) -> Vec<KindleVolume> {
    let mut found: Vec<KindleVolume> = roots
        .iter()
        .flat_map(|(root, depth)| candidates(root, *depth))
        .filter_map(|dir| inspect(&dir))
        .collect();
    found.sort_by(|a, b| a.root.cmp(&b.root));
    found.dedup_by(|a, b| a.root == b.root);
    found
}

/// `dir` and every folder up to `depth` levels below it. Unreadable folders are skipped.
fn candidates(dir: &Path, depth: usize) -> Vec<PathBuf> {
    let mut out = vec![dir.to_path_buf()];
    if depth == 0 {
        return out;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(candidates(&path, depth - 1));
            }
        }
    }
    out
}

/// `dir` as a Kindle when it has a `documents/` folder, whatever its name: for a mount point
/// the user picked themselves.
pub fn kindle_at(dir: &Path) -> Option<KindleVolume> {
    Some(KindleVolume {
        name: volume_name(dir),
        root: dir.to_path_buf(),
        documents: child_dir(dir, "documents")?,
        ejectable: crate::send::eject::ejectable(dir),
    })
}

fn inspect(dir: &Path) -> Option<KindleVolume> {
    let documents = child_dir(dir, "documents")?;
    let has_system = child_dir(dir, "system").is_some();
    if !looks_like_kindle(dir, true, has_system) {
        return None;
    }
    Some(KindleVolume {
        name: volume_name(dir),
        root: dir.to_path_buf(),
        documents,
        ejectable: crate::send::eject::ejectable(dir),
    })
}

/// The folder called `name` in `dir`, ignoring case (FAT volumes keep whatever case was used).
fn child_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir() && p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

/// The volume's folder name, or for an MTP storage ("Internal Storage") its device's name.
fn volume_name(dir: &Path) -> String {
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    let own = name(dir).unwrap_or_else(|| dir.display().to_string());
    match dir.parent().and_then(name) {
        Some(parent) if parent.starts_with("mtp:host=") => {
            parent.trim_start_matches("mtp:host=").replace('_', " ")
        }
        _ => own,
    }
}
