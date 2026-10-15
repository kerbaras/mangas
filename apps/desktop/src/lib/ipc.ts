import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirrors `mangas_core::domain::Manga`. */
export type Manga = {
  id: string;
  source: string;
  title: string;
  description: string;
  cover_url: string | null;
  status: "ongoing" | "completed" | "hiatus" | "cancelled" | "unknown";
};

/** Mirrors `mangas_core::domain::Chapter`. */
export type Chapter = {
  id: string;
  manga_id: string;
  title: string | null;
  language: string;
  volume: string | null;
  /** Chapter number as published ("12", "12.5"). */
  number: string;
  /** Directory with the downloaded pages, once downloaded. */
  local_path: string | null;
};

/** Mirrors `mangas_core::download::Event` (serde tag = "kind"). */
export type DownloadEvent =
  | { kind: "page_done"; chapter: string; page: number; total: number }
  | { kind: "chapter_done"; chapter: string; path: string; pages: number }
  | { kind: "error"; chapter: string; message: string };

/** Mirrors `mangas_core::export::kindle::Device`. */
export type Device = {
  id: string;
  name: string;
  width: number;
  height: number;
  dpi: number;
  grayscale: boolean;
};

export const DEFAULT_SOURCE = "mangadex";
export const DEFAULT_LANGUAGE = "en";
export const DOWNLOAD_EVENT = "download://progress";

export const listLibrary = () => invoke<Manga[]>("list_library");
export const listDevices = () => invoke<Device[]>("list_devices");

export const searchManga = (query: string, limit = 20, source = DEFAULT_SOURCE) =>
  invoke<Manga[]>("search_manga", { source, query, limit });

export const followManga = (id: string, source = DEFAULT_SOURCE, language = DEFAULT_LANGUAGE) =>
  invoke<Manga>("follow_manga", { source, id, language });

/** Resolves to whether the manga was in the library. */
export const unfollowManga = (id: string, source = DEFAULT_SOURCE) =>
  invoke<boolean>("unfollow_manga", { source, id });

export const listChapters = (
  mangaId: string,
  options: { source?: string; language?: string; refresh?: boolean } = {},
) =>
  invoke<Chapter[]>("list_chapters", {
    source: options.source ?? DEFAULT_SOURCE,
    mangaId,
    language: options.language ?? DEFAULT_LANGUAGE,
    refresh: options.refresh ?? false,
  });

/** Resolves to the ids of chapters that failed; progress arrives via `onDownloadProgress`. */
export const downloadChapters = (mangaId: string, chapterIds: string[], source = DEFAULT_SOURCE) =>
  invoke<string[]>("download_chapters", { source, mangaId, chapterIds });

export const onDownloadProgress = (handler: (event: DownloadEvent) => void): Promise<UnlistenFn> =>
  listen<DownloadEvent>(DOWNLOAD_EVENT, (e) => handler(e.payload));

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
