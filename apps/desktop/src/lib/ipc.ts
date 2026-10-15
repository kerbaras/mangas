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
