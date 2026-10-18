import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/format";
import { type SearchGroup, searchAll } from "@/lib/ipc";
import { recordStatus } from "@/lib/provider-status";
import type { GroupState } from "@/lib/search-groups";
import { cachedResults, rememberResults } from "@/lib/search-session";

const failed = (provider: string, reason: string): SearchGroup => ({
  provider,
  status: { state: "unreachable", reason },
  items: [],
});

/**
 * Searches each provider in its own call so groups show up as they answer instead of waiting
 * for the slowest; `retry(id)` reruns one. Only the latest call per provider lands, and every
 * answer is recorded as that provider's status. The last query's groups are kept in
 * `search-session`, so coming back to the same query shows them again; only providers still
 * searching when the screen was left are asked again.
 */
export function useMultiSearch(query: string, ids: string[]) {
  const [groups, setGroups] = useState<Record<string, GroupState>>({});
  const generation = useRef(0);
  const latest = useRef<Record<string, number>>({});
  const key = ids.join(",");
  const current = useRef({ query: "", key: "" });

  const searchOne = useCallback((q: string, id: string, gen: number) => {
    const token = (latest.current[id] ?? 0) + 1;
    latest.current[id] = token;
    setGroups((g) => ({ ...g, [id]: { kind: "searching" } }));
    searchAll(q, [id])
      .then((list) => list.find((g) => g.provider === id) ?? failed(id, "provider is disabled"))
      .catch((e: unknown) => failed(id, errorMessage(e)))
      .then((group) => {
        if (generation.current !== gen || latest.current[id] !== token) return;
        recordStatus(id, group.status);
        setGroups((g) => ({ ...g, [id]: { kind: "done", group } }));
      });
  }, []);

  useEffect(() => {
    const gen = ++generation.current;
    const q = query.trim();
    current.current = { query: q, key };
    const cached = cachedResults(q, key);
    const ids = key.split(",").filter(Boolean);
    setGroups(cached ?? {});
    if (!q) return;
    for (const id of ids) if (cached?.[id]?.kind !== "done") searchOne(q, id, gen);
  }, [query, key, searchOne]);

  // Every groups change past the reset above belongs to `current`: stale answers are dropped.
  useEffect(() => {
    rememberResults(current.current.query, current.current.key, groups);
  }, [groups]);

  const retry = useCallback(
    (id: string) => {
      const q = query.trim();
      if (q) searchOne(q, id, generation.current);
    },
    [query, searchOne],
  );
  return { groups, retry };
}
