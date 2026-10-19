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
export type ExportProgress = {
  stage: string;
  current: number;
  total: number | null;
  /** Shown once as a toast, e.g. that missing detector support is downloading first. */
  notice?: string;
};

export type ExportRequest = {
  chapter_dirs: string[];
  device: string;
  format: ExportFormat;
  preset: Preset;
  detectors: boolean;
  output: string;
  title?: string;
  /** Write over a file already at `output` instead of a ` (2)` copy. */
  replace?: boolean;
};

export type ExportResult = {
  output: string;
  pages: number;
  skipped: number;
  /** Why the detectors were asked for but the book was cut without them. */
  detectors_error?: string | null;
};
