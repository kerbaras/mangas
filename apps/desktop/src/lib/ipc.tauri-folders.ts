/** Tauri adapter for `open_folder`. */
import { invoke } from "@tauri-apps/api/core";
import type { FoldersBackend } from "./ipc.folders";

export const tauriFolders: FoldersBackend = {
  openFolder: (path) => invoke<void>("open_folder", { path }),
};
