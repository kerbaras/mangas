import { useCallback, useEffect, useRef, useState } from "react";
import type { LibraryEntry } from "@/lib/ipc.library";
import { setChapterGroup } from "@/lib/ipc.updates";
import { notifyError } from "@/lib/notify";

/**
 * The group Details narrows a title's chapters to. Followed titles start from the saved
 * choice and save every pick (one after another, so the last pick is stored), including one
 * made before following. A change saved elsewhere (the CLI) wins once no save is pending.
 */
export function useChapterGroup(entry: LibraryEntry | null, onSaved: () => void) {
  const [picked, setPicked] = useState<string | null | undefined>(undefined);
  const saved = entry ? (entry.chapter_group ?? null) : undefined;
  const group = picked === undefined ? (saved ?? null) : picked;
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const pending = useRef(0);
  const source = entry?.source;
  const id = entry?.id;

  const save = useCallback(
    (s: string, i: string, next: string | null) => {
      pending.current += 1;
      chain.current = chain.current
        .then(() => setChapterGroup(s, i, next))
        .catch((e: unknown) => notifyError("Couldn't save the group", e))
        .finally(() => {
          pending.current -= 1;
          if (pending.current === 0) onSaved();
        });
    },
    [onSaved],
  );

  const followed = useRef(source !== undefined);
  useEffect(() => {
    const now = source !== undefined && id !== undefined;
    if (now && !followed.current && picked !== undefined) save(source, id, picked);
    followed.current = now;
  }, [source, id, picked, save]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: runs when the stored value changes
  useEffect(() => {
    if (pending.current === 0) setPicked(undefined);
  }, [saved]);

  const change = useCallback(
    (next: string | null) => {
      setPicked(next);
      if (source !== undefined && id !== undefined) save(source, id, next);
    },
    [source, id, save],
  );
  return { group, setGroup: change };
}
