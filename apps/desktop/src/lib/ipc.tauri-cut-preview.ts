/** Tauri adapters for series export presets and the smart-cut preview. */
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { CutPreviewBackend } from "./ipc.cut-preview";

type Preview = {
  pages: { path: string; width: number; height: number }[];
  more: boolean;
  elapsed_ms: number;
};

export const tauriCutPreview: CutPreviewBackend = {
  getExportPreset: (source, mangaId) => invoke("get_export_preset", { source, mangaId }),
  setExportPreset: (source, mangaId, preset) =>
    invoke("set_export_preset", { source, mangaId, preset }),
  clearExportPreset: (source, mangaId) => invoke("clear_export_preset", { source, mangaId }),
  cancelCutPreview: (panel) => invoke("cancel_cut_preview", { panel }),
  previewCut: async (chapterDir, options, detectors, panel) => {
    const p = await invoke<Preview>("preview_cut", { chapterDir, options, detectors, panel });
    return {
      more: p.more,
      elapsed_ms: p.elapsed_ms,
      pages: p.pages.map((page) => ({ ...page, src: convertFileSrc(page.path) })),
    };
  },
};
