//! What a copy needs from one storage of an MTP device: list, upload, delete, rename. The real
//! device (`mtp-rs`, feature `mtp`) and the folder-backed fake both implement it, so the copy
//! protocol in `transfer` is tested without hardware.

use std::ops::ControlFlow;
use std::path::Path;

/// An object on the device (MTP object handle).
pub type Handle = u64;

/// A file or folder on the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub handle: Handle,
    pub name: String,
    pub size: u64,
    pub folder: bool,
}

/// Why a device operation failed, in the words a copy reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The user cancelled (the progress callback said stop).
    Cancelled,
    /// The storage is full.
    Full,
    /// Unplugged or stopped answering.
    Disconnected,
    /// Read-only storage or a refused write.
    Denied,
    /// The device can't do this (rename on some devices).
    Unsupported,
    /// Anything else, as the device library said it.
    Other(String),
}

/// A failed upload, with the object the device may still hold (USB backend), to delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadFailed {
    pub fault: Fault,
    pub partial: Option<Handle>,
}

/// Bytes uploaded so far; `Break` cancels the upload.
pub type OnChunk<'a> = &'a mut (dyn FnMut(u64) -> ControlFlow<()> + Send);

/// One storage of a connected MTP device.
#[async_trait::async_trait]
pub trait MtpStore: Send + Sync {
    /// The device's name, as the user knows it ("Kindle Paperwhite").
    fn name(&self) -> &str;
    /// Whether objects can be renamed (MTP `ObjectFileName`).
    fn can_rename(&self) -> bool;
    /// The objects in `parent` (the storage root when `None`).
    async fn list(&self, parent: Option<Handle>) -> Result<Vec<Entry>, Fault>;
    async fn create_folder(&self, parent: Option<Handle>, name: &str) -> Result<Handle, Fault>;
    /// Uploads `file` (`size` bytes) as `name` into `parent`.
    async fn upload(
        &self,
        parent: Handle,
        name: &str,
        file: &Path,
        size: u64,
        on_chunk: OnChunk<'_>,
    ) -> Result<Handle, UploadFailed>;
    async fn delete(&self, handle: Handle) -> Result<(), Fault>;
    async fn rename(&self, handle: Handle, name: &str) -> Result<(), Fault>;
}
