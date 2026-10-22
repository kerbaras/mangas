//! Plug and Play half of the Windows eject: find the disk's device node and ask for its removal
//! (`CM_Request_Device_EjectW`), as Explorer's "Safely remove" does.

use std::{mem, ptr};

use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Parent, CM_Request_Device_EjectW, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    CR_SUCCESS, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
};
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Ioctl::GUID_DEVINTERFACE_DISK;

use super::plan::pnp_veto;
use super::windows::{device_number, Handle};
use super::StepError;

/// Asks Plug and Play to remove the disk with `number` (its USB parent first, as Explorer
/// does). A veto for open handles is busy; any other veto is the last refusal, in Windows' words.
pub(super) fn remove_device(number: (u32, u32)) -> Result<(), StepError> {
    let disk = disk_instance(number)
        .ok_or_else(|| StepError::Failed("Windows couldn't find the drive's device".into()))?;
    let mut parent = 0u32;
    // SAFETY: plain out-parameter.
    let has_parent = unsafe { CM_Get_Parent(&mut parent, disk, 0) } == CR_SUCCESS;
    let targets = if has_parent {
        vec![parent, disk]
    } else {
        vec![disk]
    };
    let mut refused = StepError::Failed("Windows refused to remove the drive".into());
    for target in targets {
        let mut veto = 0;
        let mut name = [0u16; 260];
        // SAFETY: `name` is a writable buffer of the length passed.
        let cr = unsafe {
            CM_Request_Device_EjectW(target, &mut veto, name.as_mut_ptr(), name.len() as u32, 0)
        };
        if cr == CR_SUCCESS && veto == 0 {
            return Ok(());
        }
        log::debug!("CM_Request_Device_EjectW: cr {cr}, veto {veto}");
        refused = pnp_veto(cr, veto, &String::from_utf16_lossy(&name));
        if matches!(refused, StepError::Busy(_)) {
            break;
        }
    }
    Err(refused)
}

/// The device instance of the disk whose device number is `number`.
fn disk_instance(number: (u32, u32)) -> Option<u32> {
    // SAFETY: SetupDi calls over a device-info list destroyed before returning; each detail
    // buffer is allocated with the size the first call reports.
    unsafe {
        let set = SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_DISK,
            ptr::null(),
            ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if set as isize == INVALID_HANDLE_VALUE as isize {
            return None;
        }
        let mut found = None;
        for index in 0.. {
            let mut iface: SP_DEVICE_INTERFACE_DATA = mem::zeroed();
            iface.cbSize = mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            if SetupDiEnumDeviceInterfaces(
                set,
                ptr::null(),
                &GUID_DEVINTERFACE_DISK,
                index,
                &mut iface,
            ) == 0
            {
                break;
            }
            let mut size = 0u32;
            SetupDiGetDeviceInterfaceDetailW(
                set,
                &iface,
                ptr::null_mut(),
                0,
                &mut size,
                ptr::null_mut(),
            );
            let mut buf = vec![0u64; (size as usize).div_ceil(8).max(1)];
            let detail = buf.as_mut_ptr().cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
            (*detail).cbSize = mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            let mut info: SP_DEVINFO_DATA = mem::zeroed();
            info.cbSize = mem::size_of::<SP_DEVINFO_DATA>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(
                set,
                &iface,
                detail,
                size,
                ptr::null_mut(),
                &mut info,
            ) == 0
            {
                continue;
            }
            let path = ptr::addr_of!((*detail).DevicePath).cast::<u16>();
            let len = (0..).take_while(|&i| *path.add(i) != 0).count();
            let path = String::from_utf16_lossy(std::slice::from_raw_parts(path, len));
            let same = Handle::open(&path, 0).ok().and_then(|h| device_number(&h)) == Some(number);
            if same {
                found = Some(info.DevInst);
                break;
            }
        }
        SetupDiDestroyDeviceInfoList(set);
        found
    }
}
