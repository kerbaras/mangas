/**
 * IPC contract between the screens and the Tauri shell. Every call goes through `backend()`:
 * `ipc.tauri.ts` inside Tauri, `ipc.mock.ts` in a plain browser (`pnpm dev:web`).
 * Types here are what the screens consume; the Rust wire shapes live in `ipc.wire.ts`.
 */

import type { AppUpdateBackend } from "./ipc.app-update";
import type { CutPreviewBackend } from "./ipc.cut-preview";
import type { DetectorsBackend } from "./ipc.detectors";
import type { DiagnosticsBackend } from "./ipc.diagnostics";
import type {
  Chapter,
  ImportReport,
  Manga,
  ProviderInfo,
  ProviderStatus,
  SearchGroup,
} from "./ipc.domain";
import type { DownloadsBackend, Enqueued } from "./ipc.downloads";
import type { Device, ExportProgress, ExportRequest, ExportResult } from "./ipc.export";
import type { FoldersBackend } from "./ipc.folders";
import type { LibraryBackend } from "./ipc.library";
import type { SendBackend } from "./ipc.send";
import type { ProxyMode, Settings } from "./ipc.settings";
import type { StorageBackend } from "./ipc.storage";
import type { UpdatesBackend } from "./ipc.updates";
import type { VolumesBackend } from "./ipc.volumes";

export type { DetectorsStatus } from "./ipc.detectors";
export * from "./ipc.domain";
export type {
  Device,
  ExportFormat,
  ExportProgress,
  ExportRequest,
  ExportResult,
} from "./ipc.export";
export type { ProviderSession, ProxyMode, Settings, Theme } from "./ipc.settings";

export type Unlisten = () => void;

export type Backend = AppUpdateBackend &
  CutPreviewBackend &
  DetectorsBackend &
  DiagnosticsBackend &
  DownloadsBackend &
  FoldersBackend &
  LibraryBackend &
  SendBackend &
  StorageBackend &
  UpdatesBackend &
  VolumesBackend & {
    listLibrary(): Promise<Manga[]>;
    listDevices(): Promise<Device[]>;
    getSettings(): Promise<Settings>;
    /** Platform data folder used when no `data_dir` setting is stored. */
    defaultDataDir(): Promise<string>;
    /**
     * Saves `settings`, edited on the copy `base` loaded, over what is stored now: only the
     * fields that differ from `base` change. Resolves to the stored settings afterwards.
     */
    setSettings(settings: Settings, base?: Settings): Promise<Settings>;
    /** Links the executable as `mangas` on PATH; resolves to the link path. */
    installCli(): Promise<string>;
    listProviders(): Promise<ProviderInfo[]>;
    /** Opens the provider's site in a window; the session is stored once the check is passed. */
    connectProvider(id: string): Promise<void>;
    disconnectProvider(id: string): Promise<void>;
    /** Fires when a Connect window closes or its page fails to load (TLS, DNS). */
    onProviderConnectEnd(cb: (end: ConnectEnd) => void): Promise<Unlisten>;
    /** One cheap request with the stored session: ready, needs a session, or unreachable. */
    providerStatus(id: string): Promise<ProviderStatus>;
    /** MangaDex's status probe through a proxy setting that isn't saved yet. */
    testProxy(mode: ProxyMode, url: string): Promise<ProviderStatus>;
    /** Whether downloads go through this proxy setting (the custom one, or the system's). */
    proxyInUse(mode: ProxyMode, url: string): Promise<boolean>;
    /** Searches the enabled providers (only `providers` when not empty), one group each. */
    searchAll(query: string, providers: string[]): Promise<SearchGroup[]>;
    /** Displayable URL of the manga's cover (cached through its provider), or null. */
    coverSrc(manga: Pick<Manga, "source" | "id" | "cover_url">): Promise<string | null>;
    followManga(source: string, id: string): Promise<Manga>;
    /** The stored record when followed, otherwise the source's full details. */
    getManga(source: string, id: string): Promise<Manga>;
    unfollowManga(source: string, id: string): Promise<void>;
    listChapters(source: string, mangaId: string, refresh?: boolean): Promise<Chapter[]>;
    /** Follows the manga if needed and queues the chapters; `force` re-downloads ones on disk. */
    downloadChapters(s: string, mangaId: string, ids: string[], force?: boolean): Promise<Enqueued>;
    /** Imports chapter folders / .cbz files (or one series folder) as a `local` manga. */
    importLocal(paths: string[], title?: string): Promise<ImportReport>;
    /** Resolves to the EPUB written, its page count and the files skipped as unsupported. */
    exportChapters(request: ExportRequest): Promise<ExportResult>;
    onExportProgress(cb: (p: ExportProgress) => void): Promise<Unlisten>;
  };

/** Fallback chapter language: imports and an empty `preferred_languages`. */
export const DEFAULT_LANGUAGE = "en";

export const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let current: Backend | null = null;

/** Resolves lazily so only the bridge that applies gets pulled in. */
export async function backend(): Promise<Backend> {
  if (current) return current;
  const loaded = isTauri()
    ? (await import("./ipc.tauri")).tauriBackend
    : (await import("./ipc.mock")).mockBackend;
  // A backend pinned with `setBackend` while the import was pending wins.
  current ??= loaded;
  return current;
}

/** Tests and stories can pin a backend explicitly. */
export function setBackend(b: Backend | null) {
  current = b;
}

export const listLibrary = async () => (await backend()).listLibrary();
export const listDevices = async () => (await backend()).listDevices();
export const getSettings = async () => (await backend()).getSettings();
export const defaultDataDir = async () => (await backend()).defaultDataDir();
export const setSettings = async (s: Settings, base?: Settings) =>
  (await backend()).setSettings(s, base);
export const installCli = async () => (await backend()).installCli();
export const listProviders = async () => (await backend()).listProviders();
export const connectProvider = async (id: string) => (await backend()).connectProvider(id);
export const disconnectProvider = async (id: string) => (await backend()).disconnectProvider(id);
export const onProviderConnectEnd = async (cb: (end: ConnectEnd) => void) =>
  (await backend()).onProviderConnectEnd(cb);

/** Mirrors `commands::connect_window::ConnectEnd`. */
export type ConnectEnd =
  | { kind: "closed"; provider: string }
  | { kind: "failed"; provider: string; reason: string };
export const providerStatus = async (id: string) => (await backend()).providerStatus(id);
export const testProxy = async (mode: ProxyMode, url: string) =>
  (await backend()).testProxy(mode, url);
export const proxyInUse = async (mode: ProxyMode, url: string) =>
  (await backend()).proxyInUse(mode, url);
export const searchAll = async (q: string, providers: string[]) =>
  (await backend()).searchAll(q, providers);
export const coverSrc = async (m: Pick<Manga, "source" | "id" | "cover_url">) =>
  (await backend()).coverSrc(m);
export const followManga = async (s: string, id: string) => (await backend()).followManga(s, id);
export const getManga = async (s: string, id: string) => (await backend()).getManga(s, id);
export const unfollowManga = async (s: string, id: string) =>
  (await backend()).unfollowManga(s, id);
export const listChapters = async (s: string, id: string, refresh = false) =>
  (await backend()).listChapters(s, id, refresh);
export const downloadChapters = async (s: string, mangaId: string, ids: string[], force = false) =>
  (await backend()).downloadChapters(s, mangaId, ids, force);
export const importLocal = async (paths: string[], title?: string) =>
  (await backend()).importLocal(paths, title);
export const exportChapters = async (r: ExportRequest) => (await backend()).exportChapters(r);
export const detectorsStatus = async () => (await backend()).detectorsStatus();
export const onExportProgress = async (cb: (p: ExportProgress) => void) =>
  (await backend()).onExportProgress(cb);
