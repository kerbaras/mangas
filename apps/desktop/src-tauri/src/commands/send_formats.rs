//! What each format can go through, asked by the UI before it offers a delivery: core's USB
//! format table and email's EPUB/PDF rule, so the webview keeps no format list of its own.

use std::path::PathBuf;

use mangas_core::send::{email, usb};

/// What copying `file` onto the Kindle named `device` does, by the format table
/// (`usb::SIDELOAD`): asked before a copy, to warn or refuse with the reason.
#[tauri::command]
pub fn usb_verdict(file: PathBuf, device: String) -> usb::UsbVerdict {
    usb::usb_verdict(&file, &device)
}

/// Why Send to Kindle by email won't take `file` (it takes EPUB and PDF), or None when it will.
#[tauri::command]
pub fn email_refusal(file: String) -> Option<String> {
    email::email_refusal(&file)
}
