import { useCallback, useState } from "react";
import type { LibraryEntry } from "@/lib/ipc.library";
import { setChapterGroup } from "@/lib/ipc.updates";
import { notifyError } from "@/lib/notify";

/**
 * The group Details narrows a title's chapters to. Followed titles start from the saved
 * choice and save every change; other titles keep it for the visit only.
 */
export function useChapterGroup(entry: LibraryEntry | null, onSaved: () => void) {
  const [picked, setPicked] = useState<string | null | undefined>(undefined);
  const group = picked === undefined ? (entry?.chapter_group ?? null) : picked;
  const change = useCallback(
    (next: string | null) => {
      setPicked(next);
      if (!entry) return;
      setChapterGroup(entry.source, entry.id, next)
        .then(onSaved)
        .catch((e: unknown) => notifyError("Couldn't save the group", e));
    },
    [entry, onSaved],
  );
  return { group, setGroup: change };
}
