/** Tauri adapters for the Send to Kindle commands (`commands/send.rs`). */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CopyProgress, SendBackend } from "./ipc.send";

/** `commands::send::SEND_EVENT`. */
const SEND_EVENT = "send-progress";

export const tauriSend: SendBackend = {
  findKindles: () => invoke("find_kindles"),
  usbKindles: () => invoke("usb_kindles"),
  usbVerdict: (file, device) => invoke("usb_verdict", { file, device }),
  emailRefusal: (file) => invoke("email_refusal", { file }),
  sendUsb: (file, title, root, force) =>
    invoke("send_usb", { file, title, root: root ?? null, force: force ?? false }),
  ejectKindle: (root, files) => invoke("eject_kindle", { root, files }),
  onCopyProgress: (cb) => listen<CopyProgress>(SEND_EVENT, (e) => cb(e.payload)),
  sendEmail: (file, title) => invoke("send_email", { file, title }),
  sendTestEmail: (settings, typed) => invoke("send_test_email", { settings, typed }),
  smtpPasswordStatus: (retry) => invoke("smtp_password_status", { retry }),
  setSmtpPassword: (password) => invoke("set_smtp_password", { password }),
  listExports: (limit) => invoke("list_exports", { limit }),
};
