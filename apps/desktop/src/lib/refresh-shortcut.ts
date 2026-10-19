import { useEffect } from "react";

/** What `⌘R` refreshes: the screen showing a title registers it while mounted. */
let current: (() => void) | null = null;

export function refreshCurrent(): boolean {
  current?.();
  return current !== null;
}

export function useRefreshShortcut(refresh: () => void) {
  useEffect(() => {
    current = refresh;
    return () => {
      if (current === refresh) current = null;
    };
  }, [refresh]);
}
