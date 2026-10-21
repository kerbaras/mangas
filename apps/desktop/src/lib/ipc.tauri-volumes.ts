/** Tauri adapters for the volume export commands (`commands/volumes.rs`). */
import { invoke } from "@tauri-apps/api/core";
import { outcomeResults } from "./export-stopped";
import type { VolumeRequest, VolumesBackend } from "./ipc.volumes";
import type { VolumeOutcome } from "./ipc.wire";

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
    const outcome = await invoke<VolumeOutcome>("export_volumes", {
      ...planArgs(r),
      device: r.device,
      format: r.format,
      preset: r.preset,
      detectors: r.detectors,
      outputDir: r.outputDir,
      replace: r.replace ?? false,
      quality: r.quality ?? null,
      splitMb: r.splitMb ?? null,
      direction: r.direction ?? null,
    });
    return outcomeResults(outcome);
  },
  freeExportNames: (dir, names) => invoke("free_export_names", { dir, names }),
  checkExportDir: (dir, split, replace) => invoke("check_export_dir", { dir, split, replace }),
  exportFileName: (title, format) => invoke("export_file_name", { title, format }),
  estimateExport: (r) => invoke("estimate_export", { ...r }),
};
