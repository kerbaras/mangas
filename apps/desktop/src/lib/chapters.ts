import type { Chapter } from "./ipc";

/** Natural numeric order, so "2" sorts before "10" and "12.5" lands between 12 and 13. */
const num = (s: string | null) => {
  const n = Number.parseFloat(s ?? "");
  return Number.isNaN(n) ? Number.POSITIVE_INFINITY : n;
};

export type ChapterSort = "asc" | "desc";

export function sortChapters(chapters: Chapter[], order: ChapterSort = "asc"): Chapter[] {
  const sign = order === "asc" ? 1 : -1;
  return [...chapters].sort(
    (a, b) =>
      sign * (num(a.volume) - num(b.volume) || num(a.number) - num(b.number)) ||
      a.language.localeCompare(b.language),
  );
}

export const languagesOf = (chapters: Chapter[]): string[] =>
  [...new Set(chapters.map((c) => c.language))].sort();

export const filterByLanguage = (chapters: Chapter[], language: string | null): Chapter[] =>
  language ? chapters.filter((c) => c.language === language) : chapters;

export function chapterLabel(c: Chapter): string {
  const parts = [c.volume ? `Vol. ${c.volume}` : null, `Ch. ${c.number}`, c.title];
  return parts.filter(Boolean).join(" · ");
}

export type Selection = ReadonlySet<string>;

export const toggle = (selected: Selection, id: string): Set<string> => {
  const next = new Set(selected);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return next;
};

/** Shift-click: select everything between the anchor and the target in the visible order. */
export function selectRange(
  selected: Selection,
  visibleIds: string[],
  anchor: string | null,
  target: string,
): Set<string> {
  const from = anchor ? visibleIds.indexOf(anchor) : -1;
  const to = visibleIds.indexOf(target);
  if (from < 0 || to < 0) return toggle(selected, target);
  const [lo, hi] = from < to ? [from, to] : [to, from];
  return new Set([...selected, ...visibleIds.slice(lo, hi + 1)]);
}

/** Header checkbox: clear when everything visible is selected, otherwise select all visible. */
export function toggleAll(selected: Selection, visibleIds: string[]): Set<string> {
  const allOn = visibleIds.length > 0 && visibleIds.every((id) => selected.has(id));
  const next = new Set(selected);
  for (const id of visibleIds) {
    if (allOn) next.delete(id);
    else next.add(id);
  }
  return next;
}

export const isDownloaded = (c: Chapter) => c.local_path !== null;

export const pickSelected = (chapters: Chapter[], selected: Selection) =>
  chapters.filter((c) => selected.has(c.id));

/** What "Download selected" acts on: selected chapters that are not on disk yet. */
export const downloadable = (chapters: Chapter[], selected: Selection) =>
  pickSelected(chapters, selected).filter((c) => !isDownloaded(c));

/** What "Send to Kindle" acts on: selected chapters that are already on disk. */
export const exportable = (chapters: Chapter[], selected: Selection) =>
  pickSelected(chapters, selected).filter(isDownloaded);

export function selectionSummary(chapters: Chapter[], selected: Selection): string {
  const n = pickSelected(chapters, selected).length;
  if (n === 0) return "No chapters selected";
  const toGet = downloadable(chapters, selected).length;
  const base = `${n} selected`;
  return toGet === 0 ? `${base}, all downloaded` : `${base}, ${toGet} to download`;
}
