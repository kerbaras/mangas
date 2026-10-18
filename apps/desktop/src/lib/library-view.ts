/** Pure filtering, sorting and persistence of the library screen's view choices. */
import { type LibraryEntry, READING_STATUSES, type ReadingStatus } from "./ipc.library";

export type LibrarySort = "title" | "updated" | "unread" | "added";
export type LibraryLayout = "grid" | "list";

export type LibraryView = {
  /** Category id, or null for every title. */
  category: number | null;
  source: string | null;
  status: ReadingStatus | null;
  sort: LibrarySort;
  layout: LibraryLayout;
};

export const DEFAULT_VIEW: LibraryView = {
  category: null,
  source: null,
  status: null,
  sort: "title",
  layout: "grid",
};

export const SORTS: { id: LibrarySort; label: string }[] = [
  { id: "title", label: "Title" },
  { id: "updated", label: "Last updated" },
  { id: "unread", label: "Unread" },
  { id: "added", label: "Date added" },
];

const oneOf = <T extends string>(value: unknown, allowed: readonly T[]): T | undefined =>
  allowed.find((a) => a === value);

/** A stored view, falling back field by field to the defaults for anything unknown. */
export function parseView(raw: string | null): LibraryView {
  let stored: Record<string, unknown> = {};
  try {
    const parsed: unknown = raw ? JSON.parse(raw) : {};
    if (parsed && typeof parsed === "object") stored = parsed as Record<string, unknown>;
  } catch {
    return DEFAULT_VIEW;
  }
  const statuses = READING_STATUSES.map((s) => s.id);
  return {
    category: typeof stored.category === "number" ? stored.category : null,
    source: typeof stored.source === "string" ? stored.source : null,
    status: oneOf(stored.status, statuses) ?? null,
    sort:
      oneOf(
        stored.sort,
        SORTS.map((s) => s.id),
      ) ?? DEFAULT_VIEW.sort,
    layout: oneOf(stored.layout, ["grid", "list"] as const) ?? DEFAULT_VIEW.layout,
  };
}

export function filterEntries(
  entries: LibraryEntry[],
  view: LibraryView,
  query: string,
): LibraryEntry[] {
  const q = query.trim().toLowerCase();
  return entries.filter(
    (e) =>
      (view.category === null || e.categories.includes(view.category)) &&
      (view.source === null || e.source === view.source) &&
      (view.status === null || e.reading_status === view.status) &&
      (!q || e.title.toLowerCase().includes(q)),
  );
}

const byTitle = (a: LibraryEntry, b: LibraryEntry) => a.title.localeCompare(b.title);

/** Title A–Z; the others newest or most unread first, ties by title. */
export function sortEntries(entries: LibraryEntry[], sort: LibrarySort): LibraryEntry[] {
  const key: Record<LibrarySort, (a: LibraryEntry, b: LibraryEntry) => number> = {
    title: () => 0,
    updated: (a, b) => b.updated_at.localeCompare(a.updated_at),
    unread: (a, b) => b.chapters.unread - a.chapters.unread,
    added: (a, b) => b.added_at.localeCompare(a.added_at),
  };
  return [...entries].sort((a, b) => key[sort](a, b) || byTitle(a, b));
}

export const sourcesOf = (entries: LibraryEntry[]) =>
  [...new Set(entries.map((e) => e.source))].sort();

/** The view with filters that no longer match anything in the library dropped. */
export function liveView(view: LibraryView, entries: LibraryEntry[], categories: number[]) {
  return {
    ...view,
    category: view.category !== null && categories.includes(view.category) ? view.category : null,
    source: view.source !== null && sourcesOf(entries).includes(view.source) ? view.source : null,
  };
}
