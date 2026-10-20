/**
 * Shapes exactly as the Rust side serialises them, plus the pure mappings into the simpler
 * shapes the screens consume. Keeping the two apart means a Rust rename touches one file.
 */
import type {
  DetectorsStatus,
  ExportFormat,
  ExportProgress,
  ExportRequest,
  ExportResult,
  Preset,
} from "./ipc";
import type { RuntimeState } from "./ipc.detectors";
import { formatBytes } from "./storage";

/** Mirrors `mangas_core::export::kindle::Progress` (serde tag = "kind"). */
export type ExportStage =
  | { kind: "chapter"; index: number; total: number; path: string }
  | { kind: "pages"; done: number }
  | { kind: "writing"; pages: number };

/** Payload of the `export-progress` event (outer tag = "event"). */
export type ExportEvent =
  | { event: "model_download"; id: string; done: number; total: number | null }
  | { event: "detectors_missing"; bytes: number }
  | ({ event: "export" } & ExportStage)
  | { event: "volume"; index: number; total: number; title: string };

/** Mirrors `mangas_core::export::kindle::ExportStats`. */
export type ExportStats = {
  pages: number;
  strips: number;
  source_rows: number;
  pagination_ms: number;
  filters_ms: number;
  /** Files left out because their format cannot be decoded. */
  skipped: number;
  /** File actually written; differs from the request when that name was taken. */
  output: string;
  /** Every part in order when the book was split, else empty. */
  parts?: { path: string; pages: number }[];
  detectors_error?: string | null;
  /** Files written over the Send-to-Kindle email limit, measured on disk. */
  oversized?: { path: string; bytes: number }[];
};

export const toExportResult = (s: ExportStats): ExportResult => ({
  output: s.output,
  parts: (s.parts ?? []).map((p) => p.path),
  pages: s.pages,
  skipped: s.skipped ?? 0,
  detectors_error: s.detectors_error ?? null,
  oversized: (s.oversized ?? []).map((o) => ({ file: o.path, bytes: o.bytes })),
});

/** Mirrors `mangas_core::export::volume::VolumeOutcome`: a run stops at the first failed book. */
export type VolumeOutcome = {
  written: ExportStats[];
  failed: { title: string; output: string; reason: string } | null;
  not_written: { title: string; output: string }[];
};

/** Mirrors `mangas_core::ml::Status`. */
export type MlStatus = {
  enabled: boolean;
  downloaded: boolean;
  runtime: RuntimeState;
  models: { id: string; downloaded: boolean; licence: string }[];
  execution_provider: string | null;
  download_bytes: number;
  disk_bytes: number;
};

/** Arguments of the `export_chapters` command (camelCase on the wire, Tauri's default). */
export type ExportChaptersArgs = {
  chapterDirs: string[];
  device: string;
  format: ExportFormat;
  preset: Preset;
  detectors: boolean;
  output: string;
  title?: string;
  replace: boolean;
  quality: number | null;
  splitMb: number | null;
};

export const toExportProgress = (e: ExportEvent): ExportProgress => {
  if (e.event === "detectors_missing") {
    const notice = `Detector support was missing; downloading ${formatBytes(e.bytes)}…`;
    return { stage: notice, current: 0, total: null, notice };
  }
  if (e.event === "model_download") {
    return { stage: `Downloading detector ${e.id}`, current: e.done, total: e.total };
  }
  if (e.event === "volume") {
    return { stage: `Book ${e.index + 1} of ${e.total}: ${e.title}`, current: 0, total: null };
  }
  switch (e.kind) {
    case "chapter":
      return { stage: "Reading chapters", current: e.index + 1, total: e.total };
    case "pages":
      return { stage: "Cutting pages", current: e.done, total: null };
    case "writing":
      return { stage: "Writing file", current: e.pages, total: e.pages };
  }
};

export const toDetectorsStatus = (s: MlStatus): DetectorsStatus => ({
  available: s.enabled,
  downloaded: s.downloaded,
  provider: s.execution_provider ?? "cpu",
  runtime: s.runtime,
  downloadBytes: s.download_bytes,
  diskBytes: s.disk_bytes,
});

export const toExportArgs = (r: ExportRequest): ExportChaptersArgs => ({
  chapterDirs: r.chapter_dirs,
  device: r.device,
  format: r.format,
  preset: r.preset,
  detectors: r.detectors,
  output: r.output,
  ...(r.title ? { title: r.title } : {}),
  replace: r.replace ?? false,
  quality: r.quality ?? null,
  splitMb: r.splitMb ?? null,
});
