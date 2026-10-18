/**
 * Shapes exactly as the Rust side serialises them, plus the pure mappings into the simpler
 * shapes the screens consume. Keeping the two apart means a Rust rename touches one file.
 */
import type { DetectorsStatus, ExportProgress, ExportRequest, ExportResult, Preset } from "./ipc";

/** Mirrors `mangas_core::export::kindle::Progress` (serde tag = "kind"). */
export type KindleProgress =
  | { kind: "chapter"; index: number; total: number; path: string }
  | { kind: "pages"; done: number }
  | { kind: "writing"; pages: number };

/** Payload of the `kindle-progress` event (outer tag = "event"). */
export type KindleEvent =
  | { event: "model_download"; id: string; done: number; total: number | null }
  | ({ event: "export" } & KindleProgress);

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
};

export const toExportResult = ({ output, pages, skipped }: ExportStats): ExportResult => ({
  output,
  pages,
  skipped: skipped ?? 0,
});

/** Mirrors `mangas_core::ml::Status`. */
export type MlStatus = {
  enabled: boolean;
  downloaded: boolean;
  models: { id: string; downloaded: boolean; licence: string }[];
  execution_provider: string | null;
};

/** Arguments of the `export_kindle` command (camelCase on the wire, Tauri's default). */
export type ExportKindleArgs = {
  chapterDirs: string[];
  device: string;
  preset: Preset;
  detectors: boolean;
  output: string;
  title?: string;
};

export const toExportProgress = (e: KindleEvent): ExportProgress => {
  if (e.event === "model_download") {
    return { stage: `Downloading detector ${e.id}`, current: e.done, total: e.total };
  }
  switch (e.kind) {
    case "chapter":
      return { stage: "Reading chapters", current: e.index + 1, total: e.total };
    case "pages":
      return { stage: "Cutting pages", current: e.done, total: null };
    case "writing":
      return { stage: "Writing EPUB", current: e.pages, total: e.pages };
  }
};

export const toDetectorsStatus = (s: MlStatus): DetectorsStatus => ({
  available: s.enabled,
  downloaded: s.downloaded,
  provider: s.execution_provider ?? "cpu",
});

export const toExportArgs = (r: ExportRequest): ExportKindleArgs => ({
  chapterDirs: r.chapter_dirs,
  device: r.device,
  preset: r.preset,
  detectors: r.detectors,
  output: r.output,
  ...(r.title ? { title: r.title } : {}),
});
