import { invoke } from "@tauri-apps/api/core";

/** Mirrors `mangas_core::domain::Manga`. */
export type Manga = {
  id: string;
  source: string;
  title: string;
  description: string;
  cover_url: string | null;
  status: "ongoing" | "completed" | "hiatus" | "cancelled" | "unknown";
};

/** Mirrors `mangas_core::export::kindle::Device`. */
export type Device = {
  id: string;
  name: string;
  width: number;
  height: number;
  dpi: number;
  grayscale: boolean;
};

export const listLibrary = () => invoke<Manga[]>("list_library");
export const listDevices = () => invoke<Device[]>("list_devices");

/** Mirrors `mangas_core::smartcut::Preset`. */
export type CutPreset = "balanced" | "fewest-cuts" | "no-shrink";

/** Mirrors `mangas_core::export::kindle::ExportStats`. */
export type ExportStats = {
  pages: number;
  strips: number;
  source_rows: number;
  pagination_ms: number;
  filters_ms: number;
};

/** Mirrors `mangas_core::export::kindle::Progress`. */
export type ExportProgress =
  | { kind: "chapter"; index: number; total: number; path: string }
  | { kind: "pages"; done: number }
  | { kind: "writing"; pages: number };

/** Payload of the `kindle-progress` event emitted during `exportKindle`. */
export type KindleProgressEvent =
  | {
      event: "model_download";
      id: string;
      done: number;
      total: number | null;
    }
  | ({ event: "export" } & ExportProgress);

export const KINDLE_PROGRESS_EVENT = "kindle-progress";

/** Mirrors `mangas_core::ml::Status`. */
export type DetectorsStatus = {
  enabled: boolean;
  downloaded: boolean;
  models: { id: string; downloaded: boolean; licence: string }[];
  execution_provider: string | null;
};

export type ExportKindleArgs = {
  chapterDirs: string[];
  device: string;
  preset: CutPreset;
  detectors: boolean;
  output: string;
  title?: string;
};

export const exportKindle = (args: ExportKindleArgs) => invoke<ExportStats>("export_kindle", args);
export const detectorsStatus = () => invoke<DetectorsStatus>("detectors_status");
