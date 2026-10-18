/** The real bridge: Tauri `invoke` plus event listeners, adapted to the `Backend` contract. */
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Backend,
  Chapter,
  Device,
  ImportReport,
  Manga,
  ProviderInfo,
  ProviderStatus,
  SearchGroup,
  Settings,
} from "./ipc";
import { DEFAULT_LANGUAGE } from "./ipc";
import type { Diagnostics } from "./ipc.diagnostics";
import type { Enqueued } from "./ipc.downloads";
import type { Category, LibraryEntry } from "./ipc.library";
import { tauriDownloads } from "./ipc.tauri-downloads";
import { tauriFolders } from "./ipc.tauri-folders";
import { tauriSend } from "./ipc.tauri-send";
import { tauriStorage } from "./ipc.tauri-storage";
import { tauriUpdates } from "./ipc.tauri-updates";
import { tauriVolumes } from "./ipc.tauri-volumes";
import {
  type ExportStats,
  type ExportEvent,
  type MlStatus,
  toDetectorsStatus,
  toExportArgs,
  toExportProgress,
  toExportResult,
} from "./ipc.wire";
import { languagesFor } from "./provider-prefs";
import { readSettings } from "./settings";

export const EXPORT_EVENT = "export-progress";
const SEARCH_LIMIT = 20;

/** Chapters are fetched and stored per language: one call per language preferred for `source`. */
const languages = (source: string) => languagesFor(readSettings(), source);
const uniqueById = (lists: Chapter[][]) => [
  ...new Map(lists.flat().map((c) => [c.id, c])).values(),
];

export const tauriBackend: Backend = {
  listLibrary: () => invoke<Manga[]>("list_library"),
  listDevices: () => invoke<Device[]>("list_devices"),
  getSettings: () => invoke<Settings>("get_settings"),
  defaultDataDir: () => invoke<string>("default_data_dir"),
  setSettings: (settings, base) =>
    invoke<Settings>("set_settings", { settings, base: base ?? null }),
  installCli: () => invoke<string>("install_cli"),
  listProviders: () => invoke<ProviderInfo[]>("list_providers"),
  connectProvider: (provider) => invoke<void>("connect_provider", { provider }),
  disconnectProvider: (provider) => invoke<void>("disconnect_provider", { provider }),
  providerStatus: (provider) => invoke<ProviderStatus>("provider_status", { provider }),
  searchAll: (query, providers) =>
    invoke<SearchGroup[]>("search_all", { query, providers, limit: SEARCH_LIMIT }),
  coverSrc: async ({ source, id, cover_url }) =>
    cover_url
      ? convertFileSrc(await invoke<string>("cover_path", { source, mangaId: id, url: cover_url }))
      : null,
  followManga: async (source, id) => {
    const [first, ...rest] = languages(source);
    const manga = await invoke<Manga>("follow_manga", { source, id, language: first });
    // Extra languages are best effort: the manga is followed once the first one succeeds.
    for (const language of rest)
      await invoke<Manga>("follow_manga", { source, id, language }).catch((e: unknown) =>
        console.warn(`chapters in ${language} not synced`, e),
      );
    return manga;
  },
  getManga: (source, id) => invoke<Manga>("get_manga", { source, id }),
  unfollowManga: async (source, id) => {
    await invoke<boolean>("unfollow_manga", { source, id });
  },
  listChapters: async (source, mangaId, refresh = false) =>
    uniqueById(
      await Promise.all(
        languages(source).map((language) =>
          invoke<Chapter[]>("list_chapters", { source, mangaId, language, refresh }),
        ),
      ),
    ),
  downloadChapters: (source, mangaId, chapterIds, force = false) =>
    invoke<Enqueued>("download_chapters", {
      source,
      mangaId,
      language: languages(source)[0],
      chapterIds,
      force,
    }),
  importLocal: (paths, title) =>
    invoke<ImportReport>("import_local", {
      paths,
      title: title ?? null,
      language: DEFAULT_LANGUAGE,
    }),
  exportChapters: async (request) =>
    toExportResult(await invoke<ExportStats>("export_chapters", toExportArgs(request))),
  detectorsStatus: async () => toDetectorsStatus(await invoke<MlStatus>("detectors_status")),
  listEntries: () => invoke<LibraryEntry[]>("list_entries"),
  listCategories: () => invoke<Category[]>("list_categories"),
  createCategory: (name) => invoke<Category>("create_category", { name }),
  renameCategory: (id, name) => invoke<Category>("rename_category", { id, name }),
  deleteCategory: (id) => invoke<void>("delete_category", { id }),
  setMangaCategories: (source, mangaId, categoryIds) =>
    invoke<void>("set_manga_categories", { source, mangaId, categoryIds }),
  setReadingStatus: (source, mangaId, status) =>
    invoke<void>("set_reading_status", { source, mangaId, status }),
  diagnostics: () => invoke<Diagnostics>("diagnostics"),
  reportError: (message, level = "error") => invoke<void>("log_ui_error", { message, level }),
  ...tauriStorage,
  ...tauriSend,
  ...tauriUpdates,
  ...tauriDownloads,
  ...tauriFolders,
  ...tauriVolumes,
  onExportProgress: (cb) =>
    listen<ExportEvent>(EXPORT_EVENT, (e) => cb(toExportProgress(e.payload))),
};
