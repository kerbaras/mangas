/** Tauri adapters for the update-check commands (`commands/updates.rs`). */
import { invoke } from "@tauri-apps/api/core";
import type { UpdateRun, UpdatesBackend } from "./ipc.updates";

export const tauriUpdates: UpdatesBackend = {
  checkUpdates: (background) => invoke<UpdateRun>("check_updates", { background }),
  markViewed: (source, mangaId) => invoke<boolean>("mark_viewed", { source, mangaId }),
  setAutoDownload: (source, mangaId, on) =>
    invoke<boolean>("set_auto_download", { source, mangaId, on }),
  setChapterGroup: (source, mangaId, group) =>
    invoke<boolean>("set_chapter_group", { source, mangaId, group }),
};
