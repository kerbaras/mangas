import { useEffect } from "react";
import { markViewed } from "@/lib/ipc.updates";

const mark = (source: string, id: string) =>
  markViewed(source, id).catch((e: unknown) => console.warn("badge not cleared", e));

/**
 * Clears a followed title's "new" badge when its details open, and again when they close so
 * chapters found by the refresh while open don't show as new.
 */
export function useMarkViewed(source: string, id: string, followed: boolean) {
  useEffect(() => {
    if (!followed) return;
    void mark(source, id);
    return () => void mark(source, id);
  }, [source, id, followed]);
}
