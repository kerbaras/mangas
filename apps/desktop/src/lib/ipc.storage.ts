/** Storage and backup part of the IPC contract (`mangas_core::{storage, backup}`). */
import type { Unlisten } from "./ipc";
import { backend } from "./ipc";

/** Mirrors `mangas_core::storage::TitleUsage`. */
export type TitleUsage = {
  source: string;
  manga_id: string;
  title: string;
  chapters: number;
  bytes: number;
};

/** Mirrors `mangas_core::storage::Usage`: bytes per folder, plus each title's chapters. */
export type StorageUsage = {
  data_dir: string;
  chapters: number;
  models: number;
  covers: number;
  library: number;
  logs: number;
  titles: TitleUsage[];
  /** What a killed data-folder move left in its target, until removed or reused. */
  partial_move: PartialCopy | null;
};

/** Mirrors `mangas_core::storage::PartialCopy`: the app's subfolders in an interrupted move's target. */
export type PartialCopy = { path: string; bytes: number };

export type CacheKind = "covers" | "models";
export type Cleared = { files: number; bytes: number };
/** `kept`: recorded folders outside the download layout, left on disk. */
export type Deleted = { chapters: number; bytes: number; kept: string[] };
/** `leftovers`: old folders that couldn't be deleted after the switch. */
export type Moved = { data_dir: string; files: number; bytes: number; leftovers: string[] };
export type MoveProgress = { copied: number; total: number };
export type BackupSummary = { titles: number; skipped_local: number };
export type Restored = {
  added: number;
  already: number;
  skipped: { title: string; reason: string }[];
  /** The backup's preferences were applied. */
  settings: boolean;
  /** The backup had preferences and yours were kept. */
  kept_settings: boolean;
};
/** Whose preferences a restore ends with; `auto` keeps yours unless the library is empty. */
export type RestorePreferences = "auto" | "keep" | "backup";

export type StorageBackend = {
  storageUsage(): Promise<StorageUsage>;
  /** Deletes a title's downloaded chapters; the title and its chapter list stay. */
  deleteDownloads(source: string, mangaId: string): Promise<Deleted>;
  clearCache(cache: CacheKind): Promise<Cleared>;
  /**
   * Copies chapters and models to `to`, verifies them, then switches; progress via
   * `onMoveProgress`. `replacePartial` first deletes a partial copy a killed move left there.
   */
  moveDataDir(to: string, replacePartial?: boolean): Promise<Moved>;
  /** The partial copy a killed move left in `to`, which moving there replaces. */
  moveTargetPartial(to: string): Promise<PartialCopy | null>;
  /** Deletes the partial copy listed in `StorageUsage.partial_move`. */
  removePartialMove(): Promise<PartialCopy | null>;
  onMoveProgress(cb: (p: MoveProgress) => void): Promise<Unlisten>;
  exportBackup(path: string): Promise<BackupSummary>;
  /** Reads a mangas backup or a HaruNeko bookmarks export. */
  importBackup(path: string, preferences?: RestorePreferences): Promise<Restored>;
};

export const storageUsage = async () => (await backend()).storageUsage();
export const deleteDownloads = async (source: string, mangaId: string) =>
  (await backend()).deleteDownloads(source, mangaId);
export const clearCache = async (cache: CacheKind) => (await backend()).clearCache(cache);
export const moveDataDir = async (to: string, replacePartial = false) =>
  (await backend()).moveDataDir(to, replacePartial);
export const moveTargetPartial = async (to: string) => (await backend()).moveTargetPartial(to);
export const removePartialMove = async () => (await backend()).removePartialMove();
export const onMoveProgress = async (cb: (p: MoveProgress) => void) =>
  (await backend()).onMoveProgress(cb);
export const exportBackup = async (path: string) => (await backend()).exportBackup(path);
export const importBackup = async (path: string, preferences?: RestorePreferences) =>
  (await backend()).importBackup(path, preferences);
