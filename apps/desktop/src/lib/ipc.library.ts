/** Library organisation part of the IPC contract: entries with user state, categories, status. */

import { tr } from "@/i18n/tr";
import { backend, type Manga } from "./ipc";

/** Mirrors `mangas_core::library::ReadingStatus`: the user's, not the source's publication status. */
export type ReadingStatus = "reading" | "plan_to_read" | "on_hold" | "completed" | "dropped";

export const READING_STATUSES: { id: ReadingStatus; label: string }[] = [
  { id: "reading", label: tr("reading.reading") },
  { id: "plan_to_read", label: tr("reading.planToRead") },
  { id: "on_hold", label: tr("reading.onHold") },
  { id: "completed", label: tr("reading.completed") },
  { id: "dropped", label: tr("reading.dropped") },
];

export const readingLabel = (s: ReadingStatus) =>
  READING_STATUSES.find((r) => r.id === s)?.label ?? s;

/** Mirrors `mangas_core::library::Category`. */
export type Category = { id: number; name: string };

/** Chapter numbers (each counted once across languages); exported counts as read. */
export type ChapterCounts = {
  total: number;
  downloaded: number;
  exported: number;
  unread: number;
  /** First stored after the title was last opened: the "new" badge. */
  new: number;
};

/** Mirrors `mangas_core::library::LibraryEntry`: the manga flattened plus the user's state. */
export type LibraryEntry = Manga & {
  reading_status: ReadingStatus;
  /** Category ids, ascending. */
  categories: number[];
  /** UTC timestamps, RFC 3339: when it was followed and when a sync last found a new chapter. */
  added_at: string;
  updated_at: string;
  chapters: ChapterCounts;
  /** Update checks queue this title's new chapters for download. */
  auto_download: boolean;
  /** The scanlation group Details and downloads are narrowed to; null for every group. */
  chapter_group?: string | null;
};

export type LibraryBackend = {
  listEntries(): Promise<LibraryEntry[]>;
  listCategories(): Promise<Category[]>;
  createCategory(name: string): Promise<Category>;
  renameCategory(id: number, name: string): Promise<Category>;
  deleteCategory(id: number): Promise<void>;
  /** Replaces the manga's categories with `ids`. */
  setMangaCategories(source: string, mangaId: string, ids: number[]): Promise<void>;
  setReadingStatus(source: string, mangaId: string, status: ReadingStatus): Promise<void>;
};

export const listEntries = async () => (await backend()).listEntries();
export const listCategories = async () => (await backend()).listCategories();
export const createCategory = async (name: string) => (await backend()).createCategory(name);
export const renameCategory = async (id: number, name: string) =>
  (await backend()).renameCategory(id, name);
export const deleteCategory = async (id: number) => (await backend()).deleteCategory(id);
export const setMangaCategories = async (source: string, mangaId: string, ids: number[]) =>
  (await backend()).setMangaCategories(source, mangaId, ids);
export const setReadingStatus = async (source: string, mangaId: string, s: ReadingStatus) =>
  (await backend()).setReadingStatus(source, mangaId, s);
