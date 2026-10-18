/**
 * The window coming back to the front. The CLI (or another window) may have changed the
 * library or the settings while it was in the background, so screens reload what they show.
 * Window focus rather than a file watcher: settings and library live in one SQLite database in
 * WAL mode, where a write lands in the `-wal` file and the database file's mtime doesn't move,
 * and focus is when a reload is both cheap and visible.
 */
import { useEffect } from "react";

const listeners = new Set<() => void>();
/** `focus` and `visibilitychange` usually arrive together; one reload is enough. */
const DEBOUNCE_MS = 500;

export function onAppFocus(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Tells every listener when the window is focused or shown again; returns the stop function. */
export function watchAppFocus(): () => void {
  let last = Number.NEGATIVE_INFINITY;
  const fire = () => {
    if (document.visibilityState === "hidden" || Date.now() - last < DEBOUNCE_MS) return;
    last = Date.now();
    for (const l of listeners) l();
  };
  window.addEventListener("focus", fire);
  document.addEventListener("visibilitychange", fire);
  return () => {
    window.removeEventListener("focus", fire);
    document.removeEventListener("visibilitychange", fire);
  };
}

/** Runs `reload` each time the window comes back to the front. */
export function useAppFocus(reload: () => void) {
  useEffect(() => onAppFocus(reload), [reload]);
}
