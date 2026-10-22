/** Pure helpers behind the grouped Search screen. */

import { tr } from "@/i18n/tr";
import type { SearchGroup } from "./ipc";

export type GroupState = { kind: "searching" } | { kind: "done"; group: SearchGroup };

/** Results shown per provider in the "All" view before "Show all". */
export const PREVIEW = 4;

export type Chip = { count: number | null; searching: boolean; problem: boolean };

/** What a provider's filter chip shows: a spinner, its count, or a warning. */
export function chipOf(state: GroupState | undefined): Chip {
  if (!state) return { count: null, searching: false, problem: false };
  if (state.kind === "searching") return { count: null, searching: true, problem: false };
  const ok = state.group.status.state === "ready";
  return { count: ok ? state.group.items.length : null, searching: false, problem: !ok };
}

const results_ = (count: number) => tr("results.results", { count });
const providers_ = (count: number) => tr("results.providers", { count });

/** One line above the groups: progress while searching, then totals and providers to fix. */
export function summary(states: (GroupState | undefined)[]): string {
  const started = states.filter((s): s is GroupState => s !== undefined);
  const done = started.flatMap((s) => (s.kind === "done" ? [s.group] : []));
  const results = done.reduce((n, g) => n + g.items.length, 0);
  const pending = started.length - done.length;
  const problems = done.filter((g) => g.status.state !== "ready").length;
  if (pending > 0)
    return done.length === 0
      ? tr("results.searching", { providers: providers_(pending) })
      : tr("results.soFar", { results: results_(results), pending });
  const answered = done.length - problems;
  const head = tr("results.from", { results: results_(results), providers: providers_(answered) });
  return problems ? `${head} · ${tr("results.problems", { count: problems })}` : head;
}

/** Items to render for a group: a preview in the "All" view, everything when filtered to it. */
export const shown = <T>(items: T[], filtered: boolean): T[] =>
  filtered ? items : items.slice(0, PREVIEW);
