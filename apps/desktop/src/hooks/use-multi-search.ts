import { useCallback, useEffect, useRef, useState } from "react";
import { tr } from "@/i18n/tr";
import { errorMessage } from "@/lib/format";
import { type SearchGroup, searchAll } from "@/lib/ipc";
import { isFinalError } from "@/lib/ipc.downloads";
import { recordStatus } from "@/lib/provider-status";
import type { GroupState } from "@/lib/search-groups";
import { cachedResults, rememberResults } from "@/lib/search-session";
import { settingsSettled, useSettings, useSettingsKnown } from "@/lib/settings";

const failed = (provider: string, reason: string): SearchGroup => ({
  provider,
  status: { state: "unreachable", reason },
  items: [],
});

/** A kept group worth asking again: still searching, or failed in a way only a fix outside the
 * app can change (e.g. installing certificates), which has no Try again. */
const askAgain = (state: GroupState | undefined) =>
  state?.kind !== "done" ||
  (state.group.status.state === "unreachable" && isFinalError(state.group.status.reason));

/**
 * Searches each provider in its own call so groups show up as they answer instead of waiting
 * for the slowest; `retry(id)` reruns one. Only the latest call per provider lands, and every
 * answer is recorded as that provider's status. The last query's groups are kept in
 * `search-session` under the query, provider list and search settings, so coming back to the same
 * search shows them again; only providers still searching when the screen was left are asked
 * again, and changed languages, provider options or proxy search afresh.
 */
export function useMultiSearch(query: string, ids: string[]) {
  const [groups, setGroups] = useState<Record<string, GroupState>>({});
  const generation = useRef(0);
  const latest = useRef<Record<string, number>>({});
  const [{ preferredLanguages, providerLanguages, providerOptions, proxyMode, proxyUrl }] =
    useSettings();
  const known = useSettingsKnown();
  const key = ids.join(",");
  const prefs = JSON.stringify([
    known,
    preferredLanguages,
    providerLanguages,
    providerOptions,
    proxyMode,
    proxyUrl,
  ]);
  const current = useRef({ query: "", key: "" });

  const searchOne = useCallback((q: string, id: string, gen: number) => {
    const token = (latest.current[id] ?? 0) + 1;
    latest.current[id] = token;
    setGroups((g) => ({ ...g, [id]: { kind: "searching" } }));
    const stale = () => generation.current !== gen || latest.current[id] !== token;
    // Not before the stored settings are in and saved edits landed: the backend searches with
    // them, and a search the settings change made stale meanwhile isn't sent at all.
    settingsSettled()
      .then(() => (stale() ? null : searchAll(q, [id])))
      .then((list) =>
        list === null
          ? null
          : (list.find((g) => g.provider === id) ?? failed(id, tr("search.providerDisabled"))),
      )
      .catch((e: unknown) => failed(id, errorMessage(e)))
      .then((group) => {
        if (group === null || stale()) return;
        recordStatus(id, group.status);
        setGroups((g) => ({ ...g, [id]: { kind: "done", group } }));
      });
  }, []);

  useEffect(() => {
    const gen = ++generation.current;
    const q = query.trim();
    const cacheKey = `${key}|${prefs}`;
    current.current = { query: q, key: cacheKey };
    const cached = cachedResults(q, cacheKey);
    const ids = key.split(",").filter(Boolean);
    setGroups(cached ?? {});
    if (!q) return;
    for (const id of ids) if (askAgain(cached?.[id])) searchOne(q, id, gen);
  }, [query, key, prefs, searchOne]);

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
