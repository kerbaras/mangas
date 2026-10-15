/**
 * IPC contract with the Tauri shell. Every call goes through `backend`, which is the real
 * `invoke` bridge inside Tauri and `ipc.mock.ts` in a plain browser (`pnpm dev:web`).
 *
 * Expected Rust commands (args are camelCase on the wire, Tauri's default):
 *   list_library() -> Manga[]                      list_devices() -> Device[]
 *   search_manga(query, source) -> Manga[]         follow_manga(source, id) -> Manga
 *   unfollow_manga(source, id)                     list_chapters(source, id) -> Chapter[]
 *   download_chapters(source, chapterIds) -> job id, events "download://progress"
 *   export_kindle(request: ExportRequest), events "export://progress"
 *   detectors_status() -> DetectorsStatus
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  number: string;
  local_path: string | null;
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

/** Mirrors `mangas_core::smartcut::Preset` (kebab-case on the wire). */
export type Preset = "balanced" | "fewest-cuts" | "no-shrink";

export type DownloadProgress = {
  chapter_id: string;
  page: number;
  pages: number;
  done: boolean;
  error: string | null;
};

export type ExportProgress = { stage: string; current: number; total: number };

export type ExportRequest = {
  chapter_dirs: string[];
  device: string;
  preset: Preset;
  detectors: boolean;
  output: string;
};

export type DetectorsStatus = { downloaded: boolean; provider: string };

export type Unlisten = () => void;

export type Backend = {
  listLibrary(): Promise<Manga[]>;
  listDevices(): Promise<Device[]>;
  searchManga(query: string, source: string): Promise<Manga[]>;
  followManga(source: string, id: string): Promise<Manga>;
  unfollowManga(source: string, id: string): Promise<void>;
  listChapters(source: string, id: string): Promise<Chapter[]>;
  downloadChapters(source: string, chapterIds: string[]): Promise<string>;
  exportKindle(request: ExportRequest): Promise<void>;
  detectorsStatus(): Promise<DetectorsStatus>;
  onDownloadProgress(cb: (p: DownloadProgress) => void): Promise<Unlisten>;
  onExportProgress(cb: (p: ExportProgress) => void): Promise<Unlisten>;
};

export const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const tauriBackend: Backend = {
  listLibrary: () => invoke<Manga[]>("list_library"),
  listDevices: () => invoke<Device[]>("list_devices"),
  searchManga: (query, source) => invoke<Manga[]>("search_manga", { query, source }),
  followManga: (source, id) => invoke<Manga>("follow_manga", { source, id }),
  unfollowManga: (source, id) => invoke<void>("unfollow_manga", { source, id }),
  listChapters: (source, id) => invoke<Chapter[]>("list_chapters", { source, id }),
  downloadChapters: (source, chapterIds) =>
    invoke<string>("download_chapters", { source, chapterIds }),
  exportKindle: (request) => invoke<void>("export_kindle", { request }),
  detectorsStatus: () => invoke<DetectorsStatus>("detectors_status"),
  onDownloadProgress: (cb) => listen<DownloadProgress>("download://progress", (e) => cb(e.payload)),
  onExportProgress: (cb) => listen<ExportProgress>("export://progress", (e) => cb(e.payload)),
};

let current: Backend | null = null;

/** Resolves lazily so the mock is only pulled in outside Tauri. */
export async function backend(): Promise<Backend> {
  if (current) return current;
  current = isTauri() ? tauriBackend : (await import("./ipc.mock")).mockBackend;
  return current;
}

/** Tests and stories can pin a backend explicitly. */
export function setBackend(b: Backend | null) {
  current = b;
}

export const listLibrary = async () => (await backend()).listLibrary();
export const listDevices = async () => (await backend()).listDevices();
export const searchManga = async (q: string, s: string) => (await backend()).searchManga(q, s);
export const followManga = async (s: string, id: string) => (await backend()).followManga(s, id);
export const unfollowManga = async (s: string, id: string) =>
  (await backend()).unfollowManga(s, id);
export const listChapters = async (s: string, id: string) => (await backend()).listChapters(s, id);
export const downloadChapters = async (s: string, ids: string[]) =>
  (await backend()).downloadChapters(s, ids);
export const exportKindle = async (r: ExportRequest) => (await backend()).exportKindle(r);
export const detectorsStatus = async () => (await backend()).detectorsStatus();
export const onDownloadProgress = async (cb: (p: DownloadProgress) => void) =>
  (await backend()).onDownloadProgress(cb);
export const onExportProgress = async (cb: (p: ExportProgress) => void) =>
  (await backend()).onExportProgress(cb);
