//! The real device: `mtp-rs` (pure Rust; WPD on Windows, USB on macOS). Opens the first Amazon
//! device and its first storage (storage 0). No call is ever wrapped in a timeout or dropped
//! mid-transaction, which can wedge the device until it is replugged: a cancel stops the
//! upload through its progress callback.

use std::io::Read;
use std::ops::ControlFlow;
use std::path::Path;
use std::time::Duration;

use mtp_rs::mtp::{MtpDevice, NewObjectInfo, Storage};
use mtp_rs::{Error as MtpError, ObjectHandle};

use super::store::{Entry, Fault, Handle, MtpStore, OnChunk, UploadFailed};
use crate::send::usb::AMAZON_VENDOR_ID;

const CHUNK: usize = 1 << 20;
/// Per transfer, not per copy: a large book may take minutes over USB 2.
const TIMEOUT: Duration = Duration::from_secs(60);

pub struct RealStore {
    name: String,
    rename: bool,
    // Kept open for the storage's lifetime.
    _device: MtpDevice,
    storage: Storage,
}

/// Why the Kindle couldn't be opened.
pub enum OpenFailed {
    /// Another process holds it (macOS `ptpcamerad`, Android File Transfer).
    Held,
    Other(String),
}

/// Opens the first Kindle on the bus that speaks MTP, and its storage 0.
pub async fn open(name: &str) -> Result<RealStore, OpenFailed> {
    let held = |e: MtpError| match e.is_exclusive_access() {
        true => OpenFailed::Held,
        false => OpenFailed::Other(e.to_string()),
    };
    let known = amazon_ids();
    let info = MtpDevice::list_devices_with_known(&known)
        .map_err(held)?
        .into_iter()
        .find(|d| d.vendor_id == AMAZON_VENDOR_ID)
        .ok_or_else(|| OpenFailed::Other("no Kindle answers over MTP".into()))?;
    let device = MtpDevice::builder()
        .timeout(TIMEOUT)
        .known_devices(&known)
        .open_by_location(info.location_id)
        .await
        .map_err(held)?;
    let storage = device
        .storages()
        .await
        .map_err(held)?
        .into_iter()
        .next()
        .ok_or_else(|| OpenFailed::Other("the Kindle has no storage".into()))?;
    Ok(RealStore {
        name: name.to_string(),
        rename: device.supports_rename(),
        _device: device,
        storage,
    })
}

/// Every Amazon (vendor, product) pair on the bus, so a Kindle whose descriptor doesn't say
/// MTP still opens.
fn amazon_ids() -> Vec<(u16, u16)> {
    use nusb::MaybeFuture;
    nusb::list_devices()
        .wait()
        .map(|all| {
            all.filter(|d| d.vendor_id() == AMAZON_VENDOR_ID)
                .map(|d| (d.vendor_id(), d.product_id()))
                .collect()
        })
        .unwrap_or_default()
}

fn fault(e: MtpError) -> Fault {
    match e {
        MtpError::Cancelled => Fault::Cancelled,
        MtpError::StorageFull => Fault::Full,
        MtpError::Disconnected | MtpError::NoDevice | MtpError::DeviceReset => Fault::Disconnected,
        MtpError::AccessDenied | MtpError::PermissionDenied => Fault::Denied,
        MtpError::Unsupported => Fault::Unsupported,
        e => Fault::Other(e.to_string()),
    }
}

fn parent(handle: Option<Handle>) -> Option<ObjectHandle> {
    handle.map(ObjectHandle)
}

#[async_trait::async_trait]
impl MtpStore for RealStore {
    fn name(&self) -> &str {
        &self.name
    }

    fn can_rename(&self) -> bool {
        self.rename
    }

    async fn list(&self, at: Option<Handle>) -> Result<Vec<Entry>, Fault> {
        let listed = self.storage.list_objects(parent(at)).await.map_err(fault)?;
        Ok(listed
            .into_iter()
            .map(|o| Entry {
                handle: o.handle.0,
                folder: o.is_folder(),
                name: o.filename,
                size: o.size,
            })
            .collect())
    }

    async fn create_folder(&self, at: Option<Handle>, name: &str) -> Result<Handle, Fault> {
        let made = self.storage.create_folder(parent(at), name).await;
        made.map(|h| h.0).map_err(fault)
    }

    async fn upload(
        &self,
        at: Handle,
        name: &str,
        file: &Path,
        size: u64,
        on_chunk: OnChunk<'_>,
    ) -> Result<Handle, UploadFailed> {
        let failed = |fault| UploadFailed {
            fault,
            partial: None,
        };
        let mut src = std::fs::File::open(file).map_err(|e| failed(Fault::Other(e.to_string())))?;
        let chunks = futures::stream::poll_fn(move |_| {
            let mut buf = vec![0u8; CHUNK];
            std::task::Poll::Ready(match src.read(&mut buf) {
                Ok(0) => None,
                Ok(n) => {
                    buf.truncate(n);
                    Some(Ok(bytes::Bytes::from(buf)))
                }
                Err(e) => Some(Err(e)),
            })
        });
        let progress = |p: mtp_rs::mtp::Progress| match on_chunk(p.bytes_transferred) {
            ControlFlow::Break(()) => ControlFlow::Break(()),
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
        };
        let info = NewObjectInfo::file(name, size);
        self.storage
            .upload_with_progress(Some(ObjectHandle(at)), info, chunks, progress)
            .await
            .map(|h| h.0)
            .map_err(|e| UploadFailed {
                fault: fault(e.source),
                partial: e.partial.map(|h| h.0),
            })
    }

    async fn delete(&self, handle: Handle) -> Result<(), Fault> {
        self.storage
            .delete(ObjectHandle(handle))
            .await
            .map_err(fault)
    }

    async fn rename(&self, handle: Handle, name: &str) -> Result<(), Fault> {
        let renamed = self.storage.rename(ObjectHandle(handle), name).await;
        renamed.map_err(fault)
    }
}
