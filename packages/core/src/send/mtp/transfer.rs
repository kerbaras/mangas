//! Copying a book into `documents/` on an MTP storage without ever losing the copy that was
//! there: upload `.mangas-<pid>-<n>.part`, set the old book aside as `.mangas-<pid>-<n>.old-<name>`,
//! rename the part to the book's name, then delete the old one. A failed or cancelled upload
//! deletes the part and leaves the old book as it was; a copy cut off mid-swap (unplugged,
//! killed) gets its old book back on the next copy (`docs/research/usb-delivery.md`, "Transfer").

use std::ops::ControlFlow;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::store::{Entry, Fault, Handle, MtpStore};
use crate::send::usb::Copied;

/// Where books go; created on storage 0 when missing.
pub const DOCUMENTS: &str = "documents";
/// Prefix of the objects a copy works with, so a later copy can delete what a crash left.
const SCRATCH: &str = ".mangas-";
/// Between a scratch prefix and the name of the book set aside during a swap.
const ASIDE: &str = ".old-";

/// A copy that stopped: what went wrong on the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stopped(pub Fault);

/// Copies `file` (named `name`, `size` bytes) into `documents/`. A book there with the same
/// name and size is the same book: nothing is uploaded (`already_there`). `on_progress(done,
/// total)` gets the bytes uploaded so far; `cancelled()` stops the upload at the next chunk.
pub async fn send(
    store: &dyn MtpStore,
    file: &Path,
    name: &str,
    size: u64,
    cancelled: &(dyn Fn() -> bool + Sync),
    on_progress: &mut (dyn FnMut(u64, u64) + Send),
) -> Result<Copied, Stopped> {
    let copied = |already_there| Copied {
        path: Path::new(DOCUMENTS).join(name),
        already_there,
    };
    let folder = documents(store).await?;
    let mut listed = store.list(Some(folder)).await.map_err(Stopped)?;
    if restore_aside(store, &listed).await {
        listed = store.list(Some(folder)).await.map_err(Stopped)?;
    }
    let old = listed.iter().find(|e| !e.folder && e.name == name);
    if old.is_some_and(|e| e.size == size) {
        return Ok(copied(true));
    }
    clear_scratch(store, &listed).await;
    if cancelled() {
        return Err(Stopped(Fault::Cancelled));
    }
    let renames = store.can_rename();
    if !renames {
        // Without rename the old book can't be kept: replace it the way calibre does.
        if let Some(old) = old {
            store.delete(old.handle).await.map_err(Stopped)?;
        }
    }
    let upload_as = if renames {
        scratch(".part")
    } else {
        name.to_string()
    };
    on_progress(0, size);
    let mut on_chunk = |done: u64| {
        on_progress(done, size);
        match cancelled() {
            true => ControlFlow::Break(()),
            false => ControlFlow::Continue(()),
        }
    };
    let part = match store
        .upload(folder, &upload_as, file, size, &mut on_chunk)
        .await
    {
        Ok(part) if cancelled() => {
            let _ = store.delete(part).await;
            return Err(Stopped(Fault::Cancelled));
        }
        Ok(part) => part,
        Err(failed) => {
            drop_partial(store, folder, &upload_as, failed.partial).await;
            return Err(Stopped(failed.fault));
        }
    };
    if renames {
        swap_in(store, part, old, name).await?;
    }
    Ok(copied(false))
}

/// Puts the uploaded `part` in place of `old` under `name`; on any failure the old book is
/// back under its own name and the part is gone.
async fn swap_in(
    store: &dyn MtpStore,
    part: Handle,
    old: Option<&Entry>,
    name: &str,
) -> Result<(), Stopped> {
    let aside = scratch(&format!("{ASIDE}{name}"));
    if let Some(old) = old {
        if let Err(fault) = store.rename(old.handle, &aside).await {
            let _ = store.delete(part).await;
            return Err(Stopped(fault));
        }
    }
    if let Err(fault) = store.rename(part, name).await {
        if let Some(old) = old {
            let _ = store.rename(old.handle, name).await;
        }
        let _ = store.delete(part).await;
        return Err(Stopped(fault));
    }
    if let Some(old) = old {
        if let Err(fault) = store.delete(old.handle).await {
            tracing::warn!("could not delete the replaced {aside} on the Kindle: {fault:?}");
        }
    }
    Ok(())
}

/// The `documents` folder at the storage root (any case), created when missing.
async fn documents(store: &dyn MtpStore) -> Result<Handle, Stopped> {
    let root = store.list(None).await.map_err(Stopped)?;
    match root
        .iter()
        .find(|e| e.folder && e.name.eq_ignore_ascii_case(DOCUMENTS))
    {
        Some(found) => Ok(found.handle),
        None => store.create_folder(None, DOCUMENTS).await.map_err(Stopped),
    }
}

/// Renames each book an interrupted swap set aside back to its name when nothing took that name
/// since; true when one was put back.
async fn restore_aside(store: &dyn MtpStore, listed: &[Entry]) -> bool {
    let taken = |book: &str| listed.iter().any(|e| !e.folder && e.name == book);
    let mut restored = false;
    for e in listed.iter().filter(|e| !e.folder) {
        let Some(book) = aside_of(&e.name).filter(|book| !taken(book)) else {
            continue;
        };
        match store.rename(e.handle, book).await {
            Ok(()) => restored = true,
            Err(fault) => tracing::warn!("could not restore {book} on the Kindle: {fault:?}"),
        }
    }
    restored
}

/// The book name in `.mangas-<pid>-<n>.old-<book>`.
fn aside_of(name: &str) -> Option<&str> {
    let rest = name.strip_prefix(SCRATCH)?;
    let (_, book) = rest.split_once(ASIDE)?;
    Some(book).filter(|b| !b.is_empty())
}

/// Deletes the `.part` objects an interrupted copy left, and old books set aside whose name has
/// a book again (the swap had finished). An aside book whose name is still free stays: it is
/// the only copy of that book.
async fn clear_scratch(store: &dyn MtpStore, listed: &[Entry]) {
    let taken = |book: &str| listed.iter().any(|e| !e.folder && e.name == book);
    let only_copy = |e: &Entry| aside_of(&e.name).is_some_and(|book| !taken(book));
    for e in listed
        .iter()
        .filter(|e| !e.folder && e.name.starts_with(SCRATCH) && !only_copy(e))
    {
        if let Err(fault) = store.delete(e.handle).await {
            tracing::warn!("could not delete {} left on the Kindle: {fault:?}", e.name);
        }
    }
}

/// Deletes what a failed upload left: the handle the device gave, else an object listed under
/// the name it was uploading.
async fn drop_partial(store: &dyn MtpStore, folder: Handle, name: &str, partial: Option<Handle>) {
    let handle = match partial {
        Some(h) => Some(h),
        None => store.list(Some(folder)).await.ok().and_then(|listed| {
            listed
                .into_iter()
                .find(|e| !e.folder && e.name == name)
                .map(|e| e.handle)
        }),
    };
    if let Some(handle) = handle {
        if let Err(fault) = store.delete(handle).await {
            tracing::warn!("could not delete the partial {name} on the Kindle: {fault:?}");
        }
    }
}

/// `.mangas-<pid>-<n>.<ext>`: short and unique per copy.
fn scratch(suffix: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{SCRATCH}{}-{n}{suffix}", std::process::id())
}
