//! A mounted virtual disk (VHD, VHDX) can't be removed through Plug and Play; Explorer's
//! "Eject" detaches it from its file instead, and so does this.

use std::ptr;

use windows_sys::Win32::Foundation::{
    ERROR_INSUFFICIENT_BUFFER, ERROR_VIRTDISK_NOT_VIRTUAL_DISK, HANDLE,
};
use windows_sys::Win32::Storage::Vhd::{
    DetachVirtualDisk, GetStorageDependencyInformation, OpenVirtualDisk,
    DETACH_VIRTUAL_DISK_FLAG_NONE, GET_STORAGE_DEPENDENCY_FLAG_HOST_VOLUMES,
    OPEN_VIRTUAL_DISK_FLAG_NONE, STORAGE_DEPENDENCY_INFO, STORAGE_DEPENDENCY_INFO_TYPE_2,
    STORAGE_DEPENDENCY_INFO_VERSION_2, VIRTUAL_DISK_ACCESS_DETACH, VIRTUAL_STORAGE_TYPE,
};

use super::plan::backing_path;
use super::windows::Handle;
use super::StepError;

/// The file behind a virtual disk, read while its volume is still mounted.
pub(super) struct Backing {
    path: String,
    kind: VIRTUAL_STORAGE_TYPE,
}

/// The virtual disk file `volume` lives on; `None` for a real drive.
pub(super) fn backing(volume: &Handle) -> Option<Backing> {
    let mut size = 4096u32;
    for _ in 0..2 {
        let mut buf = vec![0u64; (size as usize).div_ceil(8)];
        let info = buf.as_mut_ptr().cast::<STORAGE_DEPENDENCY_INFO>();
        let mut used = 0u32;
        // SAFETY: `buf` is `size` writable bytes aligned for the struct; `Version` tells the
        // call which entry layout to fill in.
        let rc = unsafe {
            (*info).Version = STORAGE_DEPENDENCY_INFO_VERSION_2;
            GetStorageDependencyInformation(
                volume.0,
                GET_STORAGE_DEPENDENCY_FLAG_HOST_VOLUMES,
                size,
                info,
                &mut used,
            )
        };
        match rc {
            0 => {
                // SAFETY: the call filled `NumberEntries` entries whose strings live in `buf`.
                return unsafe { nearest(info) };
            }
            ERROR_INSUFFICIENT_BUFFER if used > size => size = used,
            ERROR_VIRTDISK_NOT_VIRTUAL_DISK => return None,
            _ => {
                log::warn!("GetStorageDependencyInformation: {rc}");
                return None;
            }
        }
    }
    None
}

/// The disk's own file: the entry closest to it in a chain of differencing disks.
unsafe fn nearest(info: *const STORAGE_DEPENDENCY_INFO) -> Option<Backing> {
    let first =
        ptr::addr_of!((*info).Anonymous.Version2Entries).cast::<STORAGE_DEPENDENCY_INFO_TYPE_2>();
    let entries = std::slice::from_raw_parts(first, (*info).NumberEntries as usize);
    let entry = entries.iter().min_by_key(|e| e.AncestorLevel)?;
    let path = backing_path(
        &wide(entry.HostVolumeName),
        &wide(entry.DependentVolumeRelativePath),
    );
    Some(Backing {
        path,
        kind: entry.VirtualStorageType,
    })
}

unsafe fn wide(s: *const u16) -> String {
    if s.is_null() {
        return String::new();
    }
    let len = (0..).take_while(|&i| *s.add(i) != 0).count();
    String::from_utf16_lossy(std::slice::from_raw_parts(s, len))
}

/// Detaches the virtual disk, which removes its drive the way unplugging would.
pub(super) fn detach(backing: &Backing) -> Result<(), StepError> {
    let fail = |rc: u32| {
        let os = std::io::Error::from_raw_os_error(rc as i32);
        StepError::Failed(format!(
            "Windows couldn't detach the virtual disk {} ({os})",
            backing.path
        ))
    };
    let path: Vec<u16> = backing.path.encode_utf16().chain(Some(0)).collect();
    let mut handle: HANDLE = ptr::null_mut();
    // SAFETY: `path` is NUL-terminated; `handle` is a plain out-parameter.
    let rc = unsafe {
        OpenVirtualDisk(
            &backing.kind,
            path.as_ptr(),
            VIRTUAL_DISK_ACCESS_DETACH,
            OPEN_VIRTUAL_DISK_FLAG_NONE,
            ptr::null(),
            &mut handle,
        )
    };
    if rc != 0 {
        return Err(fail(rc));
    }
    let disk = Handle(handle);
    // SAFETY: an open virtual disk handle, closed by `disk`'s drop.
    match unsafe { DetachVirtualDisk(disk.0, DETACH_VIRTUAL_DISK_FLAG_NONE, 0) } {
        0 => Ok(()),
        rc => Err(fail(rc)),
    }
}
