/** Tauri adapters for the app self-update commands (`commands/app_update.rs`). */
import { Channel, invoke } from "@tauri-apps/api/core";
import type { AppUpdateBackend, AppUpdateStatus, InstallProgress } from "./ipc.app-update";

export const tauriAppUpdate: AppUpdateBackend = {
  checkAppUpdate: (channel) => invoke<AppUpdateStatus>("check_app_update", { channel }),
  installAppUpdate: (version, onProgress) => {
    const progress = new Channel<InstallProgress>();
    progress.onmessage = onProgress;
    return invoke<void>("install_app_update", { version, onProgress: progress });
  },
};
