import { useCallback } from "react";
import { mangaKey } from "@/lib/format";
import { unfollowManga } from "@/lib/ipc";
import {
  type Category,
  createCategory,
  deleteCategory,
  type LibraryEntry,
  listCategories,
  listEntries,
  type ReadingStatus,
  renameCategory,
  setMangaCategories,
  setReadingStatus,
} from "@/lib/ipc.library";
import { useAsync } from "./use-async";

export type Shelf = { entries: LibraryEntry[]; categories: Category[] };

const loadShelf = async (): Promise<Shelf> => {
  const [entries, categories] = await Promise.all([listEntries(), listCategories()]);
  return { entries, categories };
};

/** Followed titles with their user state, the categories, and the actions that change them. */
export function useShelf() {
  const { state, reload, patch } = useAsync(loadShelf, []);

  const patchEntry = useCallback(
    (e: LibraryEntry, fn: (e: LibraryEntry) => LibraryEntry) =>
      patch((s) => ({
        ...s,
        entries: s.entries.map((x) => (mangaKey(x) === mangaKey(e) ? fn(x) : x)),
      })),
    [patch],
  );

  const unfollow = useCallback(
    async (e: LibraryEntry) => {
      await unfollowManga(e.source, e.id);
      patch((s) => ({ ...s, entries: s.entries.filter((x) => mangaKey(x) !== mangaKey(e)) }));
    },
    [patch],
  );

  const setStatus = useCallback(
    async (e: LibraryEntry, status: ReadingStatus) => {
      await setReadingStatus(e.source, e.id, status);
      patchEntry(e, (x) => ({ ...x, reading_status: status }));
    },
    [patchEntry],
  );

  const setCategories = useCallback(
    async (e: LibraryEntry, ids: number[]) => {
      const sorted = [...new Set(ids)].sort((a, b) => a - b);
      await setMangaCategories(e.source, e.id, sorted);
      patchEntry(e, (x) => ({ ...x, categories: sorted }));
    },
    [patchEntry],
  );

  const addCategory = useCallback(
    async (name: string) => {
      const created = await createCategory(name);
      patch((s) => ({ ...s, categories: [...s.categories, created] }));
      return created;
    },
    [patch],
  );

  const renameOne = useCallback(
    async (id: number, name: string) => {
      const renamed = await renameCategory(id, name);
      patch((s) => ({
        ...s,
        categories: s.categories.map((c) => (c.id === id ? renamed : c)),
      }));
    },
    [patch],
  );

  const removeCategory = useCallback(
    async (id: number) => {
      await deleteCategory(id);
      patch((s) => ({
        categories: s.categories.filter((c) => c.id !== id),
        entries: s.entries.map((e) => ({ ...e, categories: e.categories.filter((c) => c !== id) })),
      }));
    },
    [patch],
  );

  return {
    state,
    reload,
    unfollow,
    setStatus,
    setCategories,
    addCategory,
    renameCategory: renameOne,
    removeCategory,
  };
}

export type ShelfActions = Omit<ReturnType<typeof useShelf>, "state" | "reload">;
