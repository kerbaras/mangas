import type { Chapter } from "./ipc";

/** A scanlation group of a title and how many of its chapters are listed. */
export type GroupCount = { name: string; chapters: number };

/** Groups credited in `chapters`, most chapters first, ties by name (same as core `groups_of`). */
export function groupsOf(chapters: Chapter[]): GroupCount[] {
  const counts = new Map<string, number>();
  for (const c of chapters) if (c.group) counts.set(c.group, (counts.get(c.group) ?? 0) + 1);
  return [...counts]
    .map(([name, n]) => ({ name, chapters: n }))
    .sort((a, b) => b.chapters - a.chapters || a.name.localeCompare(b.name));
}

/** The chapters of `group` (the whole label, so "A, B" is its own group); all when null. */
export const filterByGroup = (chapters: Chapter[], group: string | null): Chapter[] =>
  group ? chapters.filter((c) => c.group === group) : chapters;

const twinKey = (c: Chapter) =>
  [c.volume ?? "", c.number, c.language, c.group ?? ""].join("\u0000");

/** Rows that share volume, number, language and group with another row: same label, different upload. */
export function duplicateIds(chapters: Chapter[]): Set<string> {
  const seen = new Map<string, number>();
  for (const c of chapters) seen.set(twinKey(c), (seen.get(twinKey(c)) ?? 0) + 1);
  return new Set(chapters.filter((c) => (seen.get(twinKey(c)) ?? 0) > 1).map((c) => c.id));
}

/** Short form of the source's chapter id ("u:1833857" → "#1833857"), to tell twin rows apart. */
export function sourceRef(id: string): string {
  const raw = id.slice(id.lastIndexOf(":") + 1);
  return `#${raw.length > 8 ? raw.slice(0, 8) : raw}`;
}

/**
 * A group name split so the cell can clip the start and always show the end, which is often
 * what tells two groups apart ("A, Wicked House" vs "Wicked House").
 */
export function splitGroup(name: string, tail = 10): [string, string] {
  if (name.length <= tail + 4) return [name, ""];
  const space = name.lastIndexOf(" ");
  const at = space > 0 && name.length - space - 1 <= tail + 4 ? space + 1 : name.length - tail;
  // Spaces go to the kept end: a clipped start drops its trailing ones.
  const head = name.slice(0, at).trimEnd();
  return [head, name.slice(head.length)];
}
