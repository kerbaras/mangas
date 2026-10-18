/** Tauri adapters for the storage and backup commands (`commands/storage.rs`). */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { StorageBackend } from "./ipc.storage";

/** `commands::storage::MOVE_EVENT`. */
const MOVE_EVENT = "storage://move";

export const tauriStorage: StorageBackend = {
  storageUsage: () => invoke("storage_usage"),
  deleteDownloads: (source, mangaId) => invoke("delete_downloads", { source, mangaId }),
  clearCache: (cache) => invoke("clear_cache", { cache }),
  moveDataDir: (to) => invoke("move_data_dir", { to }),
  onMoveProgress: (cb) =>
    listen(MOVE_EVENT, (e) => cb(e.payload as { copied: number; total: number })),
  exportBackup: (path) => invoke("export_backup", { path }),
  importBackup: (path, preferences) =>
    invoke("import_backup", { path, preferences: preferences ?? null }),
};
