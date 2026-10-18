/** Export part of the IPC contract. */
import type { Preset } from "./ipc";

/** Mirrors `mangas_core::export::Format` (lowercase on the wire). */
export type ExportFormat = "epub" | "cbz" | "pdf";

/** Mirrors `mangas_core::export::Profile`; `format` is what the device reads best. */
export type Device = {
  id: string;
  name: string;
  width: number;
  height: number;
  dpi: number;
  grayscale: boolean;
  format: ExportFormat;
};

/** `total` is null while the count is not known yet (cutting pages). */
export type ExportProgress = { stage: string; current: number; total: number | null };

export type ExportRequest = {
  chapter_dirs: string[];
  device: string;
  format: ExportFormat;
  preset: Preset;
  detectors: boolean;
  output: string;
  title?: string;
};

export type ExportResult = { output: string; pages: number; skipped: number };
