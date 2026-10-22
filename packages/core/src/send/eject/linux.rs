//! udisks2 over D-Bus, as GNOME Files and KDE's Eject do: unmount the filesystem, then eject
//! the drive's media and power it off where the drive supports it.

use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::DBusError;

use super::{dbus_error, StepError};

type Props = HashMap<String, OwnedValue>;
type Objects = HashMap<OwnedObjectPath, HashMap<String, Props>>;

const DEST: &str = "org.freedesktop.UDisks2";
const FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";
const BLOCK: &str = "org.freedesktop.UDisks2.Block";
const DRIVE: &str = "org.freedesktop.UDisks2.Drive";

pub(super) fn eject(mount: &Path) -> Result<bool, StepError> {
    let bus = Connection::system()
        .map_err(|e| StepError::Unavailable(format!("no system D-Bus: {e}")))?;
    let manager = proxy(
        &bus,
        "/org/freedesktop/UDisks2",
        "org.freedesktop.DBus.ObjectManager",
    )?;
    let objects: Objects = manager.call("GetManagedObjects", &()).map_err(step)?;
    let (block, drive) = find(&objects, mount).ok_or_else(|| {
        StepError::Unavailable(format!("udisks2 doesn't manage {}", mount.display()))
    })?;
    proxy(&bus, block.as_str(), FILESYSTEM)?
        .call::<_, _, ()>("Unmount", &(no_options(),))
        .map_err(step)?;
    Ok(drive.is_some_and(|d| power_off(&bus, &objects, &d)))
}

/// The block device mounted at `mount` and its drive (`None` for a loop device).
fn find(objects: &Objects, mount: &Path) -> Option<(OwnedObjectPath, Option<OwnedObjectPath>)> {
    let want = mount.as_os_str().as_bytes();
    objects.iter().find_map(|(path, ifaces)| {
        let points = ifaces.get(FILESYSTEM)?.get("MountPoints")?;
        let points = <Vec<Vec<u8>>>::try_from(points.try_clone().ok()?).ok()?;
        let here = points.iter().any(|p| trim_nul(p) == want);
        if !here {
            return None;
        }
        let drive = ifaces
            .get(BLOCK)
            .and_then(|b| b.get("Drive"))
            .and_then(|d| OwnedObjectPath::try_from(d.try_clone().ok()?).ok())
            .filter(|d| d.as_str() != "/");
        Some((path.clone(), drive))
    })
}

/// Ejects the media and powers the drive off where it can; `true` once powered off. Failures
/// are logged: the volume is already unmounted, so it's safe to unplug either way.
fn power_off(bus: &Connection, objects: &Objects, drive: &OwnedObjectPath) -> bool {
    let props = objects.get(drive).and_then(|i| i.get(DRIVE));
    let flag = |name: &str| {
        props
            .and_then(|p| p.get(name))
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(false)
    };
    let call = |method: &str| match proxy(bus, drive.as_str(), DRIVE) {
        Ok(p) => p
            .call::<_, _, ()>(method, &(no_options(),))
            .map_err(|e| log::warn!("udisks2 {method} {}: {e}", drive.as_str()))
            .is_ok(),
        Err(e) => {
            log::warn!("udisks2 {method}: {e:?}");
            false
        }
    };
    let ejected = flag("Ejectable") && call("Eject");
    let off = flag("CanPowerOff") && call("PowerOff");
    ejected || off
}

fn proxy<'a>(bus: &Connection, path: &'a str, iface: &'a str) -> Result<Proxy<'a>, StepError> {
    Proxy::new(bus, DEST, path, iface).map_err(step)
}

fn no_options() -> HashMap<&'static str, Value<'static>> {
    HashMap::new()
}

fn trim_nul(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(&[0]).unwrap_or(bytes)
}

fn step(e: zbus::Error) -> StepError {
    match &e {
        zbus::Error::MethodError(name, message, _) => {
            dbus_error(name.as_str(), message.as_deref().unwrap_or(""))
        }
        zbus::Error::FDO(fdo) => dbus_error(fdo.name().as_str(), fdo.description().unwrap_or("")),
        _ => StepError::Failed(e.to_string()),
    }
}
