/**
 * Something in this window added or deleted files in the data folder (detector support download
 * or removal, a cache clear), so every view that shows sizes or detector status reloads.
 */
import { useEffect } from "react";

const listeners = new Set<() => void>();

export function storageChanged() {
  for (const l of listeners) l();
}

/** Runs `reload` after each [`storageChanged`]. */
export function useStorageChanged(reload: () => void) {
  useEffect(() => {
    listeners.add(reload);
    return () => {
      listeners.delete(reload);
    };
  }, [reload]);
}
