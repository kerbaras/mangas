/** Tauri adapters for the download queue commands (`commands/downloads.rs`). */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DownloadJob, DownloadsBackend, QueueEvent, QueueStatus } from "./ipc.downloads";

/** The event the queue worker emits every change on (`lib.rs`). */
export const QUEUE_EVENT = "download://queue";

export const tauriDownloads: DownloadsBackend = {
  listDownloads: () => invoke<DownloadJob[]>("list_downloads"),
  pauseDownload: (id) => invoke<DownloadJob[]>("pause_download", { id }),
  resumeDownload: (id) => invoke<DownloadJob[]>("resume_download", { id }),
  cancelDownload: (id) => invoke<DownloadJob[]>("cancel_download", { id }),
  retryDownload: (id) => invoke<DownloadJob[]>("retry_download", { id }),
  clearDownloads: () => invoke<number>("clear_downloads"),
  dismissDownload: (id) => invoke<void>("dismiss_download", { id }),
  queueStatus: () => invoke<QueueStatus>("queue_status"),
  openDownloadFolder: (id) => invoke<void>("open_download_folder", { id }),
  onQueueEvent: (cb) => listen<QueueEvent>(QUEUE_EVENT, (e) => cb(e.payload)),
};
