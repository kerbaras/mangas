//! Windows' "Safely remove": lock, dismount and eject the volume (DeviceIoControl), then ask
//! Plug and Play to remove the device (`CM_Request_Device_EjectW`), as calibre's eject does. A
//! virtual disk (VHD) is detached instead, as Explorer's "Eject" does.

use std::ffi::c_void;
use std::path::Path;
use std::{mem, ptr, thread, time::Duration};

use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Ioctl::{
    FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_STORAGE_EJECT_MEDIA,
    IOCTL_STORAGE_GET_DEVICE_NUMBER, IOCTL_STORAGE_MEDIA_REMOVAL, PREVENT_MEDIA_REMOVAL,
    STORAGE_DEVICE_NUMBER,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

use super::windows_pnp::remove_device;
use super::{windows_vhd, StepError};

const LOCK_TRIES: u32 = 5;

/// `Ok(true)` once the device was removed, its media ejected or its virtual disk detached. A
/// dismounted volume that stays attached isn't enough: Windows mounts it again on the next
/// access, so that is an error naming why Windows wouldn't remove it.
pub(super) fn eject(mount: &Path) -> Result<bool, StepError> {
    let letter = mount
        .to_string_lossy()
        .chars()
        .next()
        .filter(char::is_ascii_alphabetic)
        .ok_or_else(|| StepError::Failed(format!("{} isn't a drive", mount.display())))?;
    let volume = Handle::open(&format!(r"\\.\{letter}:"), GENERIC_READ | GENERIC_WRITE)
        .map_err(|e| StepError::Failed(format!("Windows couldn't open {letter}: ({e})")))?;
    let backing = windows_vhd::backing(&volume);
    lock(&volume).map_err(|e| {
        StepError::Busy(format!(
            "a program still has a file open on {letter}:, so Windows couldn't lock it: {e}"
        ))
    })?;
    // SAFETY: a valid volume handle; flushing a locked volume is what its owner may do.
    unsafe { FlushFileBuffers(volume.0) };
    ioctl(&volume, FSCTL_DISMOUNT_VOLUME, &[])
        .map_err(|e| StepError::Failed(format!("Windows couldn't dismount {letter}: ({e})")))?;
    // SAFETY: zeroed is `PreventMediaRemoval = FALSE`.
    let allow: PREVENT_MEDIA_REMOVAL = unsafe { mem::zeroed() };
    let _ = ioctl(&volume, IOCTL_STORAGE_MEDIA_REMOVAL, as_bytes(&allow));
    let media = ioctl(&volume, IOCTL_STORAGE_EJECT_MEDIA, &[]).is_ok();
    let number = device_number(&volume);
    drop(volume);
    let removed = number.map_or_else(
        || {
            Err(StepError::Failed(
                "Windows couldn't find the drive's device".into(),
            ))
        },
        remove_device,
    );
    match (removed, backing) {
        (Ok(()), _) => Ok(true),
        (Err(busy @ StepError::Busy(_)), _) => Err(busy),
        (Err(_), Some(vhd)) => windows_vhd::detach(&vhd).map(|()| true),
        // A card reader stays plugged in, but with its media out there's no volume left.
        (Err(_), None) if media && std::fs::metadata(format!(r"{letter}:\")).is_err() => Ok(true),
        (Err(e), None) => Err(e),
    }
}

pub(super) struct Handle(pub(super) HANDLE);

impl Handle {
    pub(super) fn open(path: &str, access: u32) -> std::io::Result<Handle> {
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        // SAFETY: `wide` is NUL-terminated; the other arguments are plain flags.
        let h = unsafe {
            CreateFileW(
                wide.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if h == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Handle(h))
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateFileW and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

fn lock(volume: &Handle) -> std::io::Result<()> {
    let mut last = Ok(());
    for _ in 0..LOCK_TRIES {
        last = ioctl(volume, FSCTL_LOCK_VOLUME, &[]);
        if last.is_ok() {
            return last;
        }
        thread::sleep(Duration::from_millis(200));
    }
    last
}

fn ioctl(h: &Handle, code: u32, input: &[u8]) -> std::io::Result<()> {
    let mut returned = 0u32;
    let (ptr_in, len) = match input {
        [] => (ptr::null(), 0),
        bytes => (bytes.as_ptr().cast::<c_void>(), bytes.len() as u32),
    };
    // SAFETY: the input buffer outlives the call; no output buffer is asked for.
    let ok = unsafe {
        DeviceIoControl(
            h.0,
            code,
            ptr_in,
            len,
            ptr::null_mut(),
            0,
            &mut returned,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn as_bytes<T>(v: &T) -> &[u8] {
    // SAFETY: a plain-old-data struct viewed as its bytes for the call's lifetime.
    unsafe { std::slice::from_raw_parts((v as *const T).cast::<u8>(), mem::size_of::<T>()) }
}

pub(super) fn device_number(h: &Handle) -> Option<(u32, u32)> {
    // SAFETY: STORAGE_DEVICE_NUMBER is plain data the ioctl fills in.
    let mut n: STORAGE_DEVICE_NUMBER = unsafe { mem::zeroed() };
    let mut returned = 0u32;
    let ok = unsafe {
        DeviceIoControl(
            h.0,
            IOCTL_STORAGE_GET_DEVICE_NUMBER,
            ptr::null(),
            0,
            (&mut n as *mut STORAGE_DEVICE_NUMBER).cast(),
            mem::size_of::<STORAGE_DEVICE_NUMBER>() as u32,
            &mut returned,
            ptr::null_mut(),
        )
    };
    (ok != 0).then_some((n.DeviceType, n.DeviceNumber))
}
