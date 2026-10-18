/** Tauri adapters for the detector support commands (`commands/detectors.rs`). */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DetectorsBackend, DetectorsProgress } from "./ipc.detectors";
import { type MlStatus, toDetectorsStatus } from "./ipc.wire";

/** `commands::detectors::DETECTORS_EVENT`. */
const DETECTORS_EVENT = "detectors://download";

export const tauriDetectors: DetectorsBackend = {
  detectorsStatus: async () => toDetectorsStatus(await invoke<MlStatus>("detectors_status")),
  downloadDetectors: async () => toDetectorsStatus(await invoke<MlStatus>("download_detectors")),
  cancelDetectorsDownload: () => invoke("cancel_detectors_download"),
  removeDetectors: () => invoke<number>("remove_detectors"),
  onDetectorsProgress: (cb) => listen<DetectorsProgress>(DETECTORS_EVENT, (e) => cb(e.payload)),
};
