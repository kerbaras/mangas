/** Tauri adapters for the Send to Kindle commands (`commands/send.rs`). */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CopyProgress, SendBackend } from "./ipc.send";

/** `commands::send::SEND_EVENT`. */
const SEND_EVENT = "send-progress";

export const tauriSend: SendBackend = {
  findKindles: () => invoke("find_kindles"),
  sendUsb: (file, title, root) => invoke("send_usb", { file, title, root: root ?? null }),
  onCopyProgress: (cb) => listen<CopyProgress>(SEND_EVENT, (e) => cb(e.payload)),
  sendEmail: (file, title) => invoke("send_email", { file, title }),
  sendTestEmail: (settings) => invoke("send_test_email", { settings }),
  listExports: (limit) => invoke("list_exports", { limit }),
};
