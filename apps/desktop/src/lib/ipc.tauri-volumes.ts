/** Tauri adapters for the volume export commands (`commands/volumes.rs`). */
import { invoke } from "@tauri-apps/api/core";
import type { VolumeRequest, VolumesBackend } from "./ipc.volumes";
import { type ExportStats, toExportResult } from "./ipc.wire";

const planArgs = (r: VolumeRequest) => ({
  source: r.source,
  mangaId: r.mangaId,
  chapterIds: r.chapterIds,
  grouping: r.grouping,
  title: r.title?.trim() || null,
});

export const tauriVolumes: VolumesBackend = {
  planVolumes: (r) => invoke("plan_volumes", planArgs(r)),
  exportVolumes: async (r) => {
    const stats = await invoke<ExportStats[]>("export_volumes", {
      ...planArgs(r),
      device: r.device,
      preset: r.preset,
      detectors: r.detectors,
      outputDir: r.outputDir,
    });
    return stats.map(toExportResult);
  },
};
