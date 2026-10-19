/** Series export presets and the smart-cut preview (`commands/cut_preview.rs`). */
import { backend, type Preset } from "./ipc";
import type { ExportFormat } from "./ipc.export";

/** Mirrors `mangas_core::export::ExportPreset`: every field optional, unset = the default. */
export type ExportPreset = {
  cut_preset?: Preset;
  max_shrink?: number;
  overlap?: number;
  /** Fraction of the screen height a page cut from a strip fills (0.5–1). */
  page_height?: number;
  trim_margins?: boolean;
  contrast?: number;
  gamma?: number;
  sharpen?: boolean;
  format?: ExportFormat;
  /** Profile id. */
  profile?: string;
};

export type PreviewPage = { src: string; width: number; height: number };

export type CutPreview = {
  pages: PreviewPage[];
  more: boolean;
  elapsed_ms: number;
  /** Why the detectors were asked for but the cut ran without them. */
  detectors_error?: string | null;
};

export type CutPreviewBackend = {
  getExportPreset(source: string, mangaId: string): Promise<ExportPreset | null>;
  /** An empty preset clears it. */
  setExportPreset(source: string, mangaId: string, preset: ExportPreset): Promise<void>;
  clearExportPreset(source: string, mangaId: string): Promise<boolean>;
  /** The first pages an export of the chapter would write with `options`; a newer call cancels. */
  previewCut(
    chapterDir: string,
    options: ExportPreset,
    detectors: boolean,
    panel: string,
  ): Promise<CutPreview>;
  /** Stops the previews of `panel` (closed), running or still to arrive. */
  cancelCutPreview(panel: string): Promise<void>;
};

/** Drops unset fields so an untouched form is an empty preset. */
export const compact = (p: ExportPreset): ExportPreset =>
  Object.fromEntries(
    Object.entries(p).filter(([, v]) => v !== undefined && v !== null),
  ) as ExportPreset;

export const isEmptyPreset = (p: ExportPreset | null | undefined) =>
  !p || Object.keys(compact(p)).length === 0;

export const getExportPreset = async (source: string, mangaId: string) =>
  (await backend()).getExportPreset(source, mangaId);
export const setExportPreset = async (source: string, mangaId: string, preset: ExportPreset) =>
  (await backend()).setExportPreset(source, mangaId, compact(preset));
export const clearExportPreset = async (source: string, mangaId: string) =>
  (await backend()).clearExportPreset(source, mangaId);
export const previewCut = async (
  dir: string,
  options: ExportPreset,
  detectors: boolean,
  panel: string,
) => (await backend()).previewCut(dir, compact(options), detectors, panel);
export const cancelCutPreview = async (panel: string) => (await backend()).cancelCutPreview(panel);
