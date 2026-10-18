import { useCallback, useEffect, useRef, useState } from "react";
import type { LibraryEntry } from "@/lib/ipc.library";
import { setChapterGroup } from "@/lib/ipc.updates";
import { notifyError } from "@/lib/notify";

/**
 * The group Details narrows a title's chapters to. Followed titles start from the saved
 * choice and save every change, including one picked before following. Saves run one after
 * another, so the last pick is the one stored.
 */
export function useChapterGroup(entry: LibraryEntry | null, onSaved: () => void) {
  const [picked, setPicked] = useState<string | null | undefined>(undefined);
  const saved = entry ? (entry.chapter_group ?? null) : undefined;
  const group = picked === undefined ? (saved ?? null) : picked;
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const source = entry?.source;
  const id = entry?.id;
  useEffect(() => {
    if (source === undefined || id === undefined || picked === undefined || picked === saved)
      return;
    chain.current = chain.current
      .then(() => setChapterGroup(source, id, picked))
      .then(onSaved)
      .catch((e: unknown) => notifyError("Couldn't save the group", e));
  }, [source, id, picked, saved, onSaved]);
  const change = useCallback((next: string | null) => setPicked(next), []);
  return { group, setGroup: change };
}
