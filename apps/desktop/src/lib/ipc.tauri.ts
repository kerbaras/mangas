/** The real bridge: Tauri `invoke` plus event listeners, adapted to the `Backend` contract. */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Backend, Chapter, Device, ImportReport, Manga, ProviderInfo, Settings } from "./ipc";
import {
  type DownloadEvent,
  type ExportStats,
  type KindleEvent,
  type MlStatus,
  toDetectorsStatus,
  toDownloadProgress,
  toExportArgs,
  toExportProgress,
} from "./ipc.wire";

export const DEFAULT_LANGUAGE = "en";
export const DOWNLOAD_EVENT = "download://progress";
export const KINDLE_EVENT = "kindle-progress";
const SEARCH_LIMIT = 20;

export const tauriBackend: Backend = {
  listLibrary: () => invoke<Manga[]>("list_library"),
  listDevices: () => invoke<Device[]>("list_devices"),
  getSettings: () => invoke<Settings>("get_settings"),
  defaultDataDir: () => invoke<string>("default_data_dir"),
  setSettings: (settings) => invoke<void>("set_settings", { settings }),
  installCli: () => invoke<string>("install_cli"),
  listProviders: () => invoke<ProviderInfo[]>("list_providers"),
  connectProvider: (provider) => invoke<void>("connect_provider", { provider }),
  disconnectProvider: (provider) => invoke<void>("disconnect_provider", { provider }),
  searchManga: (query, source) =>
    invoke<Manga[]>("search_manga", { source, query, limit: SEARCH_LIMIT }),
  followManga: (source, id) =>
    invoke<Manga>("follow_manga", { source, id, language: DEFAULT_LANGUAGE }),
  unfollowManga: async (source, id) => {
    await invoke<boolean>("unfollow_manga", { source, id });
  },
  listChapters: (source, mangaId, refresh = false) =>
    invoke<Chapter[]>("list_chapters", {
      source,
      mangaId,
      language: DEFAULT_LANGUAGE,
      refresh,
    }),
  downloadChapters: (source, mangaId, chapterIds) =>
    invoke<string[]>("download_chapters", {
      source,
      mangaId,
      language: DEFAULT_LANGUAGE,
      chapterIds,
    }),
  importLocal: (paths, title) =>
    invoke<ImportReport>("import_local", {
      paths,
      title: title ?? null,
      language: DEFAULT_LANGUAGE,
    }),
  exportKindle: async (request) =>
    (await invoke<ExportStats>("export_kindle", toExportArgs(request))).output,
  detectorsStatus: async () => toDetectorsStatus(await invoke<MlStatus>("detectors_status")),
  onDownloadProgress: (cb) =>
    listen<DownloadEvent>(DOWNLOAD_EVENT, (e) => cb(toDownloadProgress(e.payload))),
  onExportProgress: (cb) =>
    listen<KindleEvent>(KINDLE_EVENT, (e) => cb(toExportProgress(e.payload))),
};
