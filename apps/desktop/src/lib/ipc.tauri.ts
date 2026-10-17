/** The real bridge: Tauri `invoke` plus event listeners, adapted to the `Backend` contract. */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Backend,
  Chapter,
  Device,
  ImportReport,
  Manga,
  ProviderInfo,
  ProviderStatus,
  Settings,
} from "./ipc";
import { DEFAULT_LANGUAGE } from "./ipc";
import type { Diagnostics } from "./ipc.diagnostics";
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
import { readSettings } from "./settings";

export const DOWNLOAD_EVENT = "download://progress";
export const KINDLE_EVENT = "kindle-progress";
const SEARCH_LIMIT = 20;

/** Chapters are fetched and stored per language: one call per preferred language. */
const languages = () => readSettings().preferredLanguages;
const uniqueById = (lists: Chapter[][]) => [
  ...new Map(lists.flat().map((c) => [c.id, c])).values(),
];

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
  providerStatus: (provider) => invoke<ProviderStatus>("provider_status", { provider }),
  searchManga: (query, source) =>
    invoke<Manga[]>("search_manga", { source, query, limit: SEARCH_LIMIT }),
  followManga: async (source, id) => {
    const [first, ...rest] = languages();
    const manga = await invoke<Manga>("follow_manga", { source, id, language: first });
    for (const language of rest) await invoke<Manga>("follow_manga", { source, id, language });
    return manga;
  },
  unfollowManga: async (source, id) => {
    await invoke<boolean>("unfollow_manga", { source, id });
  },
  listChapters: async (source, mangaId, refresh = false) =>
    uniqueById(
      await Promise.all(
        languages().map((language) =>
          invoke<Chapter[]>("list_chapters", { source, mangaId, language, refresh }),
        ),
      ),
    ),
  downloadChapters: (source, mangaId, chapterIds) =>
    invoke<string[]>("download_chapters", {
      source,
      mangaId,
      language: languages()[0],
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
  diagnostics: () => invoke<Diagnostics>("diagnostics"),
  openLogs: () => invoke<void>("open_logs"),
  reportError: (message) => invoke<void>("log_ui_error", { message }),
  onDownloadProgress: (cb) =>
    listen<DownloadEvent>(DOWNLOAD_EVENT, (e) => cb(toDownloadProgress(e.payload))),
  onExportProgress: (cb) =>
    listen<KindleEvent>(KINDLE_EVENT, (e) => cb(toExportProgress(e.payload))),
};
