import { useCallback } from "react";
import { useShelf } from "./use-shelf";

/**
 * A title's library state on the details screen: its entry (null while not followed), the
 * categories, and the shelf actions. `reload` refetches after a follow or a refresh.
 */
export function useLibraryEntry(source: string, id: string) {
  const { state, reload, setStatus, setCategories, addCategory } = useShelf();
  const ready = state.kind === "ready" ? state.data : null;
  const entry = ready?.entries.find((e) => e.source === source && e.id === id) ?? null;
  const toggle = useCallback(
    (cid: number, on: boolean) => {
      if (!entry) return Promise.resolve();
      const next = on ? [...entry.categories, cid] : entry.categories.filter((c) => c !== cid);
      return setCategories(entry, next);
    },
    [entry, setCategories],
  );
  return { entry, categories: ready?.categories ?? [], reload, setStatus, toggle, addCategory };
}
