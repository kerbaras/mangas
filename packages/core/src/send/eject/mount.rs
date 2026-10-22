//! The mount point a path lives on.

use std::path::{Path, PathBuf};

/// The longest of `mounts` that `path` is inside.
pub fn containing_mount(mounts: &[PathBuf], path: &Path) -> Option<PathBuf> {
    mounts
        .iter()
        .filter(|m| path.starts_with(m))
        .max_by_key(|m| m.components().count())
        .cloned()
}

/// Mount points from `/proc/self/mountinfo` (field 5, with `\040`-style escapes).
#[cfg(target_os = "linux")]
pub(super) fn mount_point(path: &Path) -> Option<PathBuf> {
    let path = path.canonicalize().ok()?;
    let info = std::fs::read_to_string("/proc/self/mountinfo").ok()?;
    let mounts: Vec<PathBuf> = info
        .lines()
        .filter_map(|l| l.split(' ').nth(4))
        .map(|m| PathBuf::from(unescape(m)))
        .collect();
    containing_mount(&mounts, &path)
}

#[cfg(target_os = "linux")]
fn unescape(field: &str) -> String {
    let mut out = String::new();
    let mut rest = field;
    while let Some(i) = rest.find('\\') {
        out.push_str(&rest[..i]);
        let code = rest
            .get(i + 1..i + 4)
            .and_then(|o| u8::from_str_radix(o, 8).ok());
        match code {
            Some(c) => {
                out.push(char::from(c));
                rest = &rest[i + 4..];
            }
            None => {
                out.push('\\');
                rest = &rest[i + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// `statfs(2)`'s `f_mntonname`.
#[cfg(target_os = "macos")]
pub(super) fn mount_point(path: &Path) -> Option<PathBuf> {
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: `c` is a valid C string and `fs` a zeroed out-parameter statfs fills in.
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut fs) } != 0 {
        return None;
    }
    // SAFETY: statfs NUL-terminates f_mntonname.
    let name = unsafe { CStr::from_ptr(fs.f_mntonname.as_ptr()) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(name.to_bytes())))
}

/// The drive the path is on: `E:\`.
#[cfg(windows)]
pub(super) fn mount_point(path: &Path) -> Option<PathBuf> {
    use std::path::Component;
    match path.components().next()? {
        Component::Prefix(p) => Some(PathBuf::from(format!(
            "{}\\",
            p.as_os_str().to_string_lossy()
        ))),
        _ => None,
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(super) fn mount_point(_path: &Path) -> Option<PathBuf> {
    None
}
