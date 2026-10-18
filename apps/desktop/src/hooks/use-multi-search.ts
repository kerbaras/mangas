import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/format";
import { type SearchGroup, searchAll } from "@/lib/ipc";
import { recordStatus } from "@/lib/provider-status";
import type { GroupState } from "@/lib/search-groups";

const failed = (provider: string, reason: string): SearchGroup => ({
  provider,
  status: { state: "unreachable", reason },
  items: [],
});

/**
 * Searches each provider in its own call so groups show up as they answer instead of waiting
 * for the slowest; `retry(id)` reruns one. Every answer is recorded as that provider's status.
 */
export function useMultiSearch(query: string, ids: string[]) {
  const [groups, setGroups] = useState<Record<string, GroupState>>({});
  const generation = useRef(0);
  const key = ids.join(",");

  const searchOne = useCallback((q: string, id: string, gen: number) => {
    setGroups((g) => ({ ...g, [id]: { kind: "searching" } }));
    searchAll(q, [id])
      .then((list) => list.find((g) => g.provider === id) ?? failed(id, "provider is disabled"))
      .catch((e: unknown) => failed(id, errorMessage(e)))
      .then((group) => {
        if (generation.current !== gen) return;
        recordStatus(id, group.status);
        setGroups((g) => ({ ...g, [id]: { kind: "done", group } }));
      });
  }, []);

  useEffect(() => {
    const gen = ++generation.current;
    setGroups({});
    const q = query.trim();
    if (!q) return;
    for (const id of key.split(",").filter(Boolean)) searchOne(q, id, gen);
  }, [query, key, searchOne]);

  const retry = useCallback(
    (id: string) => {
      const q = query.trim();
      if (q) searchOne(q, id, generation.current);
    },
    [query, searchOne],
  );
  return { groups, retry };
}
