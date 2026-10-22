//! The pure half of an eject: which strategies an OS tries, how an OS answer is classified, and
//! what the user reads.

use std::path::Path;

use serde::Serialize;

/// Appended to every refused eject: the copy itself succeeded.
pub const STILL_THERE: &str = "The books copied to it are still there";

/// Why an MTP-only Kindle has no Eject: there's no mounted drive to remove.
pub const MTP_NO_EJECT: &str =
    "is connected over MTP, so there's nothing to eject; unplug it once the copy has finished";

/// One way to unmount and eject a volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    /// udisks2 over D-Bus: `Filesystem.Unmount`, then `Drive.Eject` / `Drive.PowerOff`.
    Udisks,
    /// `sync` + `umount`, when udisks2 isn't running.
    Umount,
    /// `diskutil eject <mount>`.
    Diskutil,
    /// Lock, dismount and eject the volume (DeviceIoControl), then `CM_Request_Device_EjectW`.
    Windows,
}

impl Strategy {
    pub fn parse(s: &str) -> Option<Strategy> {
        match s.trim() {
            "udisks" => Some(Strategy::Udisks),
            "umount" => Some(Strategy::Umount),
            "diskutil" => Some(Strategy::Diskutil),
            "windows" => Some(Strategy::Windows),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Macos,
    Windows,
    Other,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::Macos
        } else if cfg!(windows) {
            Os::Windows
        } else {
            Os::Other
        }
    }
}

/// How one OS step went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepError {
    /// The volume is in use; the OS's own words.
    Busy(String),
    /// This way can't run here (no udisks2, a volume it doesn't manage): try the next one.
    Unavailable(String),
    /// The OS refused, in its own words.
    Failed(String),
}

/// The strategies tried in order: `forced` alone, or the OS's own way first.
pub fn strategies(os: Os, forced: Option<Strategy>) -> Vec<Strategy> {
    if let Some(s) = forced {
        return vec![s];
    }
    match os {
        Os::Linux => vec![Strategy::Udisks, Strategy::Umount],
        Os::Macos => vec![Strategy::Diskutil],
        Os::Windows => vec![Strategy::Windows],
        Os::Other => vec![Strategy::Umount],
    }
}

/// `false` for a Kindle reached over MTP (a gvfs folder under `root`): nothing to eject.
pub fn ejectable(root: &Path) -> bool {
    !root
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with("mtp:host="))
}

/// A D-Bus error from udisks2, by name: busy, absent, or refused.
pub fn dbus_error(name: &str, message: &str) -> StepError {
    let text = if message.is_empty() { name } else { message }.to_string();
    match name {
        "org.freedesktop.UDisks2.Error.DeviceBusy" => StepError::Busy(text),
        "org.freedesktop.DBus.Error.ServiceUnknown"
        | "org.freedesktop.DBus.Error.NameHasNoOwner"
        | "org.freedesktop.DBus.Error.Spawn.ServiceNotFound"
        | "org.freedesktop.DBus.Error.UnknownObject"
        | "org.freedesktop.DBus.Error.UnknownMethod" => StepError::Unavailable(text),
        // Activation failures: the service is masked, disabled or failed to start.
        n if n.starts_with("org.freedesktop.DBus.Error.Spawn.")
            || n.starts_with("org.freedesktop.systemd1.") =>
        {
            StepError::Unavailable(text)
        }
        _ if text.contains("target is busy") => StepError::Busy(text),
        _ => StepError::Failed(text),
    }
}

/// The program macOS names in "dissented by PID 412 (/usr/bin/less)".
pub fn dissenter(text: &str) -> Option<String> {
    let after = text.split("dissented by PID").nth(1)?;
    let inside = after.split_once('(')?.1.split_once(')')?.0;
    let name = inside.rsplit('/').next()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Whether the Kindle at `root` is the whole volume mounted at `mount` (never the system root):
/// only then does ejecting it remove just the Kindle.
pub fn own_volume(mount: &Path, root: &Path) -> bool {
    mount.components().eq(root.components()) && mount != Path::new("/")
}

/// "can't eject KINDLE at /media/me/KINDLE: it's in use by less and vlc. Close them and eject
/// again; …", naming the OS's answer when it can't say which program.
pub fn busy_text(device: &str, holders: &[String], detail: &str) -> String {
    let who = match holders {
        [] => {
            let detail = detail.trim().trim_end_matches('.');
            return format!(
                "can't eject {device}: it's in use ({detail}). Close any window or program \
                 using files on it and eject again. {STILL_THERE}"
            );
        }
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    let them = if holders.len() == 1 { "it" } else { "them" };
    format!(
        "can't eject {device}: it's in use by {who}. Close {them} and eject again. {STILL_THERE}"
    )
}

/// What a refused `CM_Request_Device_EjectW` means: `cr` its return code, `veto` its
/// `PNP_VETO_TYPE`, `who` the vetoing program, service or driver. Only "something has it open"
/// is busy; the rest leave the drive dismounted but still attached.
pub fn pnp_veto(cr: u32, veto: i32, who: &str) -> StepError {
    let who = who.trim_end_matches('\0').trim();
    let named = |what: &str| match who {
        "" => what.to_string(),
        w => format!("{what} ({w})"),
    };
    match veto {
        2..=5 => StepError::Busy(named("a program or service still has it open")),
        8 | 10 => StepError::Failed(named("Windows can't remove this kind of drive")),
        12 => StepError::Failed(named("Windows won't let this user remove the drive")),
        0 => StepError::Failed(format!("Windows refused to remove the drive (error {cr})")),
        v => StepError::Failed(named(&format!(
            "Windows refused to remove the drive (veto {v})"
        ))),
    }
}

/// The path to a virtual disk's file: its host volume (`\\?\Volume{…}\`) plus the path
/// relative to it, as `GetStorageDependencyInformation` gives them.
pub fn backing_path(host: &str, relative: &str) -> String {
    format!(
        "{}\\{}",
        host.trim_end_matches('\\'),
        relative.trim_start_matches('\\')
    )
}
